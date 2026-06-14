use std::path::{Path, PathBuf};

/// The hidden folder that holds derived state (sqlite index, etc.).
pub const NEXUS_DIR: &str = ".nexus";
/// Marker file that turns a folder into a Notion-style database.
pub const DB_MARKER: &str = ".nexusdb.json";

/// Convert an absolute path under `root` to a vault-relative, forward-slash path.
pub fn to_rel(root: &Path, abs: &Path) -> Option<String> {
    let rel = abs.strip_prefix(root).ok()?;
    let s = rel.to_string_lossy().replace('\\', "/");
    Some(s.trim_start_matches('/').to_string())
}

/// Resolve a vault-relative path back to an absolute path.
pub fn to_abs(root: &Path, rel: &str) -> PathBuf {
    if rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(rel)
    }
}

/// Parent of a vault-relative path: `a/b/c` -> `a/b`, `a` -> ``, `` -> ``.
pub fn parent_rel(rel: &str) -> String {
    match rel.rsplit_once('/') {
        Some((p, _)) => p.to_string(),
        None => String::new(),
    }
}

/// True if any path segment is hidden (starts with `.`) — these are skipped by
/// the indexer, except the database marker file which we detect separately.
pub fn is_hidden(rel: &str) -> bool {
    rel.split('/').any(|seg| seg.starts_with('.'))
}

/// Guard against path traversal in names coming from the UI.
pub fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != ".."
}
