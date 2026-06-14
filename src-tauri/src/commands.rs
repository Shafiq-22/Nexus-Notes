//! All `#[tauri::command]` entry points. Argument names are camelCase on the JS
//! side; Tauri maps them to these snake_case Rust parameters.
use crate::error::{AppError, AppResult};
use crate::model::*;
use crate::state::AppState;
use crate::write_guard::WriteGuard;
use crate::{index, paths, vault::Vault};
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};

/// Cheap, lock-free handle to the open vault's shared resources.
struct VaultRef {
    root: PathBuf,
    name: String,
    conn: Arc<Mutex<Connection>>,
    guard: Arc<WriteGuard>,
}

fn vref(state: &AppState) -> AppResult<VaultRef> {
    let g = state.vault.lock();
    let v = g.as_ref().ok_or(AppError::NoVault)?;
    Ok(VaultRef {
        root: v.root.clone(),
        name: v.name.clone(),
        conn: v.conn.clone(),
        guard: v.guard.clone(),
    })
}

fn atomic_write(abs: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Write to a sibling temp file (non-.md, ignored by the watcher) then rename
    // so a save is a single atomic event and never a partial read.
    let tmp = abs.with_extension("nexustmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, abs)?;
    Ok(())
}

fn sanitize_filename(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let t = cleaned.trim().to_string();
    if t.is_empty() {
        "Untitled".into()
    } else {
        t
    }
}

fn unique_rel(root: &Path, dir: &str, base: &str) -> String {
    let mut name = format!("{base}.md");
    let mut i = 1;
    loop {
        let rel = if dir.is_empty() {
            name.clone()
        } else {
            format!("{dir}/{name}")
        };
        if !paths::to_abs(root, &rel).exists() {
            return rel;
        }
        i += 1;
        name = format!("{base} {i}.md");
    }
}

// --- vault lifecycle -------------------------------------------------------

#[tauri::command]
pub fn open_vault(state: State<'_, AppState>, app: AppHandle, path: String) -> AppResult<VaultInfo> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(AppError::InvalidPath);
    }
    let vault = Vault::open(root.clone(), &app)?;
    let name = vault.name.clone();
    let conn = vault.conn.clone();
    *state.vault.lock() = Some(vault);

    // Heavy initial index runs off the UI thread; progress arrives via events.
    let app2 = app.clone();
    let root2 = root.clone();
    std::thread::spawn(move || {
        let c = conn.lock();
        let _ = crate::vault::reconcile_full(&c, &root2, &app2);
    });

    Ok(VaultInfo {
        root: root.to_string_lossy().into_owned(),
        name,
        note_count: 0,
    })
}

#[tauri::command]
pub fn close_vault(state: State<'_, AppState>) -> AppResult<()> {
    *state.vault.lock() = None;
    Ok(())
}

/// A vault path supplied via the `NEXUS_VAULT` environment variable, so the app
/// (or CI/screenshot tooling) can launch straight into a vault.
#[tauri::command]
pub fn startup_vault() -> Option<String> {
    std::env::var("NEXUS_VAULT").ok().filter(|s| !s.is_empty())
}

#[tauri::command]
pub fn reindex(state: State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    let v = vref(state.inner())?;
    std::thread::spawn(move || {
        let c = v.conn.lock();
        let _ = crate::vault::reconcile_full(&c, &v.root, &app);
    });
    Ok(())
}

#[tauri::command]
pub fn get_vault_tree(state: State<'_, AppState>) -> AppResult<TreeNode> {
    let v = vref(state.inner())?;
    let c = v.conn.lock();
    index::get_tree(&c, &v.name)
}

// --- file CRUD -------------------------------------------------------------

#[tauri::command]
pub fn read_note(state: State<'_, AppState>, path: String) -> AppResult<RawNote> {
    let v = vref(state.inner())?;
    let abs = paths::to_abs(&v.root, &path);
    let content = std::fs::read_to_string(&abs)?;
    let hash = nexus_vault::hash_hex(content.as_bytes());
    let parsed = nexus_vault::split_frontmatter(&content);
    let mtime = std::fs::metadata(&abs)
        .ok()
        .as_ref()
        .map(crate::vault::mtime_ms)
        .unwrap_or(0);
    Ok(RawNote {
        path,
        markdown: parsed.body,
        frontmatter: parsed.frontmatter,
        hash,
        mtime_ms: mtime,
    })
}

#[tauri::command]
pub fn write_note(
    state: State<'_, AppState>,
    path: String,
    markdown: String,
    frontmatter: serde_json::Value,
) -> AppResult<NoteMeta> {
    let v = vref(state.inner())?;
    let abs = paths::to_abs(&v.root, &path);
    let content = nexus_vault::compose_note(&frontmatter, &markdown);
    let hash = nexus_vault::hash_hex(content.as_bytes());
    v.guard.arm(&abs, Some(hash));
    atomic_write(&abs, content.as_bytes())?;
    let meta = std::fs::metadata(&abs)?;
    let mtime = crate::vault::mtime_ms(&meta);
    let size = meta.len() as i64;
    let c = v.conn.lock();
    index::upsert_note(&c, &path, mtime, size, &content)
}

#[tauri::command]
pub fn create_note(state: State<'_, AppState>, parent: String, name: String) -> AppResult<NoteMeta> {
    let v = vref(state.inner())?;
    if !paths::is_safe_name(&name) {
        return Err(AppError::InvalidPath);
    }
    let stem = name.trim_end_matches(".md").to_string();
    let rel = if parent.is_empty() {
        format!("{stem}.md")
    } else {
        format!("{parent}/{stem}.md")
    };
    let abs = paths::to_abs(&v.root, &rel);
    if abs.exists() {
        return Err(AppError::AlreadyExists);
    }
    let content = format!("# {stem}\n\n");
    let hash = nexus_vault::hash_hex(content.as_bytes());
    v.guard.arm(&abs, Some(hash));
    atomic_write(&abs, content.as_bytes())?;
    let meta = std::fs::metadata(&abs)?;
    let c = v.conn.lock();
    index::upsert_note(&c, &rel, crate::vault::mtime_ms(&meta), meta.len() as i64, &content)
}

#[tauri::command]
pub fn create_folder(state: State<'_, AppState>, parent: String, name: String) -> AppResult<()> {
    let v = vref(state.inner())?;
    if !paths::is_safe_name(&name) {
        return Err(AppError::InvalidPath);
    }
    let rel = if parent.is_empty() {
        name.clone()
    } else {
        format!("{parent}/{name}")
    };
    let abs = paths::to_abs(&v.root, &rel);
    if abs.exists() {
        return Err(AppError::AlreadyExists);
    }
    v.guard.arm(&abs, None);
    std::fs::create_dir_all(&abs)?;
    let c = v.conn.lock();
    index::upsert_folder(&c, &rel)
}

fn do_rename(v: &VaultRef, from: &str, to: &str) -> AppResult<()> {
    let abs_from = paths::to_abs(&v.root, from);
    let abs_to = paths::to_abs(&v.root, to);
    if !abs_from.exists() {
        return Err(AppError::InvalidPath);
    }
    if abs_to.exists() {
        return Err(AppError::AlreadyExists);
    }
    v.guard.arm(&abs_from, None);
    v.guard.arm(&abs_to, None);
    if let Some(p) = abs_to.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::rename(&abs_from, &abs_to)?;
    let c = v.conn.lock();
    index::remove_path_recursive(&c, from)?;
    crate::vault::reconcile_subtree(&c, &v.root, to)?;
    index::refresh_db_membership(&c)?;
    Ok(())
}

#[tauri::command]
pub fn rename_path(state: State<'_, AppState>, from: String, to: String) -> AppResult<()> {
    let v = vref(state.inner())?;
    do_rename(&v, &from, &to)
}

#[tauri::command]
pub fn move_path(state: State<'_, AppState>, from: String, to_parent: String) -> AppResult<String> {
    let v = vref(state.inner())?;
    let name = from.rsplit('/').next().unwrap_or(&from).to_string();
    let to = if to_parent.is_empty() {
        name
    } else {
        format!("{to_parent}/{name}")
    };
    do_rename(&v, &from, &to)?;
    Ok(to)
}

#[tauri::command]
pub fn delete_path(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let v = vref(state.inner())?;
    let abs = paths::to_abs(&v.root, &path);
    v.guard.arm(&abs, None);
    if abs.is_dir() {
        std::fs::remove_dir_all(&abs)?;
    } else if abs.exists() {
        std::fs::remove_file(&abs)?;
    }
    let c = v.conn.lock();
    index::remove_path_recursive(&c, &path)
}

// --- search / index queries ------------------------------------------------

#[tauri::command]
pub fn search(state: State<'_, AppState>, query: String, limit: Option<i64>) -> AppResult<Vec<SearchHit>> {
    let v = vref(state.inner())?;
    let c = v.conn.lock();
    index::search(&c, &query, limit.unwrap_or(50))
}

#[tauri::command]
pub fn search_suggest(state: State<'_, AppState>, prefix: String) -> AppResult<Vec<Suggestion>> {
    let v = vref(state.inner())?;
    let c = v.conn.lock();
    index::search_suggest(&c, &prefix)
}

#[tauri::command]
pub fn get_backlinks(state: State<'_, AppState>, path: String) -> AppResult<Vec<Backlink>> {
    let v = vref(state.inner())?;
    let c = v.conn.lock();
    index::get_backlinks(&c, &path)
}

#[tauri::command]
pub fn get_outline(state: State<'_, AppState>, path: String) -> AppResult<Vec<Heading>> {
    let v = vref(state.inner())?;
    let abs = paths::to_abs(&v.root, &path);
    let content = std::fs::read_to_string(&abs)?;
    let body = nexus_vault::split_frontmatter(&content).body;
    Ok(nexus_vault::extract_headings(&body)
        .into_iter()
        .map(|h| Heading {
            level: h.level,
            text: h.text,
            line: h.line,
        })
        .collect())
}

#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> AppResult<Vec<TagCount>> {
    let v = vref(state.inner())?;
    let c = v.conn.lock();
    index::list_tags(&c)
}

#[tauri::command]
pub fn notes_with_tag(
    state: State<'_, AppState>,
    tag: String,
    include_subtags: bool,
) -> AppResult<Vec<SearchHit>> {
    let v = vref(state.inner())?;
    let c = v.conn.lock();
    index::notes_with_tag(&c, &tag, include_subtags)
}

// --- graph -----------------------------------------------------------------

#[tauri::command]
pub fn graph_global(state: State<'_, AppState>) -> AppResult<GraphData> {
    let v = vref(state.inner())?;
    let c = v.conn.lock();
    index::graph_global(&c)
}

#[tauri::command]
pub fn graph_local(state: State<'_, AppState>, path: String, depth: u8) -> AppResult<GraphData> {
    let v = vref(state.inner())?;
    let c = v.conn.lock();
    index::graph_local(&c, &path, depth)
}

// --- databases -------------------------------------------------------------

#[tauri::command]
pub fn list_databases(state: State<'_, AppState>) -> AppResult<Vec<DbInfo>> {
    let v = vref(state.inner())?;
    let c = v.conn.lock();
    index::list_databases(&c)
}

#[tauri::command]
pub fn get_database(state: State<'_, AppState>, path: String) -> AppResult<DbSchema> {
    let v = vref(state.inner())?;
    let c = v.conn.lock();
    index::get_database(&c, &path)
}

#[tauri::command]
pub fn query_database(state: State<'_, AppState>, path: String) -> AppResult<DbResult> {
    let v = vref(state.inner())?;
    let c = v.conn.lock();
    index::query_database(&c, &path)
}

#[tauri::command]
pub fn upsert_record(
    state: State<'_, AppState>,
    db_path: String,
    path: Option<String>,
    fields: serde_json::Value,
) -> AppResult<NoteMeta> {
    let v = vref(state.inner())?;
    let rel = match path {
        Some(p) => p,
        None => {
            let title = fields
                .get("title")
                .and_then(|x| x.as_str())
                .unwrap_or("Untitled");
            unique_rel(&v.root, &db_path, &sanitize_filename(title))
        }
    };
    let abs = paths::to_abs(&v.root, &rel);
    let (mut fm, body) = if abs.exists() {
        let content = std::fs::read_to_string(&abs)?;
        let p = nexus_vault::split_frontmatter(&content);
        (p.frontmatter, p.body)
    } else {
        (serde_json::json!({}), String::new())
    };
    if let (Some(obj), Some(newobj)) = (fm.as_object_mut(), fields.as_object()) {
        for (k, val) in newobj {
            obj.insert(k.clone(), val.clone());
        }
    }
    let content = nexus_vault::compose_note(&fm, &body);
    let hash = nexus_vault::hash_hex(content.as_bytes());
    v.guard.arm(&abs, Some(hash));
    atomic_write(&abs, content.as_bytes())?;
    let meta = std::fs::metadata(&abs)?;
    let c = v.conn.lock();
    let nm = index::upsert_note(&c, &rel, crate::vault::mtime_ms(&meta), meta.len() as i64, &content)?;
    index::refresh_db_membership(&c)?;
    Ok(nm)
}
