//! Filesystem watcher → debounce → echo-check → reconcile → emit.
//!
//! App-initiated writes update the index synchronously inside the command and
//! arm the [`WriteGuard`], so the echo that arrives here is suppressed. Only
//! genuine external changes (other editors, Finder/Explorer) flow through to
//! the frontend as `fs:*` events.
use crate::error::{AppError, AppResult};
use crate::events::{names, FsEvent};
use crate::model::NoteMeta;
use crate::vault::mtime_ms;
use crate::write_guard::WriteGuard;
use crate::{index, paths};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{
    new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap,
};
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub type Watch = Debouncer<notify::RecommendedWatcher, FileIdMap>;

pub fn start(
    root: PathBuf,
    conn: Arc<Mutex<Connection>>,
    guard: Arc<WriteGuard>,
    app: AppHandle,
) -> AppResult<Watch> {
    let watch_root = root.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        None,
        move |res: DebounceEventResult| {
            if let Ok(events) = res {
                for ev in events {
                    handle_event(&root, &conn, &guard, &app, &ev);
                }
            }
        },
    )
    .map_err(|e| AppError::Other(format!("watcher: {e}")))?;

    debouncer
        .watcher()
        .watch(&watch_root, RecursiveMode::Recursive)
        .map_err(|e| AppError::Other(format!("watch: {e}")))?;
    Ok(debouncer)
}

fn handle_event(
    root: &Path,
    conn: &Arc<Mutex<Connection>>,
    guard: &WriteGuard,
    app: &AppHandle,
    ev: &DebouncedEvent,
) {
    for path in ev.paths.iter() {
        let rel = match paths::to_rel(root, path) {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        if rel.split('/').next() == Some(paths::NEXUS_DIR) {
            continue;
        }
        let fname = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let is_marker = fname == paths::DB_MARKER;
        let is_md = rel.ends_with(".md");
        if !is_marker && paths::is_hidden(&rel) {
            continue;
        }

        if path.exists() {
            if path.is_dir() {
                if guard.should_suppress(path, None) {
                    continue;
                }
                {
                    let c = conn.lock();
                    let _ = index::upsert_folder(&c, &rel);
                }
                emit_fs(app, names::FS_CREATED, &rel, "folder", None);
            } else if is_marker {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let folder = paths::parent_rel(&rel);
                    let c = conn.lock();
                    let _ = index::register_database(&c, &folder, &content);
                    let _ = index::refresh_db_membership(&c);
                }
            } else if is_md {
                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let hash = nexus_vault::hash_hex(content.as_bytes());
                if guard.should_suppress(path, Some(&hash)) {
                    continue;
                }
                let meta = std::fs::metadata(path).ok();
                let mtime = meta.as_ref().map(mtime_ms).unwrap_or(0);
                let size = meta.as_ref().map(|m| m.len() as i64).unwrap_or(0);
                let (existed, nm) = {
                    let c = conn.lock();
                    let existed = c
                        .query_row("SELECT 1 FROM notes WHERE path=?1", [&rel], |_| Ok(()))
                        .is_ok();
                    let nm = index::upsert_note(&c, &rel, mtime, size, &content).ok();
                    (existed, nm)
                };
                emit_fs(
                    app,
                    if existed {
                        names::FS_CHANGED
                    } else {
                        names::FS_CREATED
                    },
                    &rel,
                    "note",
                    nm,
                );
            }
        } else {
            // Removed from disk.
            if guard.should_suppress(path, None) {
                continue;
            }
            let known = {
                let c = conn.lock();
                let k = c
                    .query_row(
                        "SELECT 1 FROM notes WHERE path=?1
                         UNION ALL SELECT 1 FROM folders WHERE path=?1 LIMIT 1",
                        [&rel],
                        |_| Ok(()),
                    )
                    .is_ok();
                if k {
                    let _ = if is_md {
                        index::remove_note(&c, &rel)
                    } else {
                        index::remove_path_recursive(&c, &rel)
                    };
                }
                k
            };
            if known {
                emit_fs(
                    app,
                    names::FS_DELETED,
                    &rel,
                    if is_md { "note" } else { "folder" },
                    None,
                );
            }
        }
    }
}

fn emit_fs(app: &AppHandle, name: &str, rel: &str, kind: &str, meta: Option<NoteMeta>) {
    let _ = app.emit(
        name,
        FsEvent {
            path: rel.to_string(),
            kind: kind.to_string(),
            meta,
        },
    );
}
