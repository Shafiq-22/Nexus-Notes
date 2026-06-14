use crate::error::AppResult;
use crate::events::{names, IndexProgress, IndexReady, SyncStatus};
use crate::write_guard::WriteGuard;
use crate::{fs_watch, index, paths};
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

/// An open vault: a real folder on disk plus its derived SQLite index, the
/// echo-suppression guard, and a live filesystem watcher (kept alive here).
pub struct Vault {
    pub root: PathBuf,
    pub name: String,
    pub conn: Arc<Mutex<Connection>>,
    pub guard: Arc<WriteGuard>,
    pub _watcher: fs_watch::Watch,
}

impl Vault {
    pub fn open(root: PathBuf, app: &AppHandle) -> AppResult<Vault> {
        let nexus = root.join(paths::NEXUS_DIR);
        std::fs::create_dir_all(&nexus)?;
        let conn = Connection::open(nexus.join("index.sqlite"))?;
        index::init_schema(&conn)?;
        let conn = Arc::new(Mutex::new(conn));
        let guard = Arc::new(WriteGuard::new());
        let name = root
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Vault".into());
        let watcher = fs_watch::start(root.clone(), conn.clone(), guard.clone(), app.clone())?;
        Ok(Vault {
            root,
            name,
            conn,
            guard,
            _watcher: watcher,
        })
    }
}

/// Modified-time of a file in milliseconds since the Unix epoch.
pub fn mtime_ms(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn emit_sync(app: &AppHandle, state: &str) {
    let _ = app.emit(
        names::SYNC_STATUS,
        SyncStatus {
            state: state.into(),
        },
    );
}

/// Full disk → index rebuild. Disk is the source of truth, so we clear derived
/// tables and re-walk. Emits progress + ready events for the status bar.
pub fn reconcile_full(conn: &Connection, root: &Path, app: &AppHandle) -> AppResult<usize> {
    emit_sync(app, "reconciling");
    index::clear_index(conn)?;

    let mut folders = Vec::new();
    let mut notes = Vec::new();
    let mut markers = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            if e.file_type().is_dir() {
                return !e.file_name().to_string_lossy().starts_with('.');
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if entry.depth() == 0 {
            continue;
        }
        let rel = match paths::to_rel(root, entry.path()) {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        if entry.file_type().is_dir() {
            folders.push(rel);
        } else {
            let fname = entry.file_name().to_string_lossy();
            if fname == paths::DB_MARKER {
                markers.push((paths::parent_rel(&rel), entry.path().to_path_buf()));
            } else if rel.ends_with(".md") && !paths::is_hidden(&rel) {
                notes.push((rel, entry.path().to_path_buf()));
            }
        }
    }

    for f in &folders {
        index::upsert_folder(conn, f)?;
    }
    for (folder, path) in &markers {
        if let Ok(content) = std::fs::read_to_string(path) {
            let _ = index::register_database(conn, folder, &content);
        }
    }

    let total = notes.len();
    for (i, (rel, abs)) in notes.iter().enumerate() {
        if let Ok(meta) = std::fs::metadata(abs) {
            if let Ok(content) = std::fs::read_to_string(abs) {
                let _ = index::upsert_note(conn, rel, mtime_ms(&meta), meta.len() as i64, &content);
            }
        }
        if i % 50 == 0 || i + 1 == total {
            let _ = app.emit(
                names::INDEX_PROGRESS,
                IndexProgress {
                    done: i + 1,
                    total,
                    phase: "indexing".into(),
                },
            );
        }
    }

    index::refresh_db_membership(conn)?;
    let count = index::count_notes(conn)?;
    let _ = app.emit(names::INDEX_READY, IndexReady { note_count: count });
    emit_sync(app, "watching");
    Ok(count)
}

/// Re-index a single path (file or directory subtree) after an app-initiated
/// rename/move.
pub fn reconcile_subtree(conn: &Connection, root: &Path, rel: &str) -> AppResult<()> {
    let abs = paths::to_abs(root, rel);
    if abs.is_dir() {
        for entry in WalkDir::new(&abs)
            .into_iter()
            .filter_entry(|e| {
                !(e.file_type().is_dir() && e.file_name().to_string_lossy().starts_with('.'))
            })
            .filter_map(|e| e.ok())
        {
            let erel = match paths::to_rel(root, entry.path()) {
                Some(r) if !r.is_empty() => r,
                _ => continue,
            };
            if entry.file_type().is_dir() {
                index::upsert_folder(conn, &erel)?;
            } else {
                let fname = entry.file_name().to_string_lossy();
                if fname == paths::DB_MARKER {
                    if let Ok(c) = std::fs::read_to_string(entry.path()) {
                        let _ = index::register_database(conn, &paths::parent_rel(&erel), &c);
                    }
                } else if erel.ends_with(".md") && !paths::is_hidden(&erel) {
                    if let (Ok(meta), Ok(content)) = (
                        std::fs::metadata(entry.path()),
                        std::fs::read_to_string(entry.path()),
                    ) {
                        let _ =
                            index::upsert_note(conn, &erel, mtime_ms(&meta), meta.len() as i64, &content);
                    }
                }
            }
        }
    } else if rel.ends_with(".md") {
        if let (Ok(meta), Ok(content)) =
            (std::fs::metadata(&abs), std::fs::read_to_string(&abs))
        {
            index::upsert_note(conn, rel, mtime_ms(&meta), meta.len() as i64, &content)?;
        }
    }
    Ok(())
}
