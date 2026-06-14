//! SQLite metadata index (via rusqlite) + full-text search via FTS5.
//!
//! Disk is the source of truth; this index is derived and can be rebuilt at any
//! time by deleting `.nexus/index.sqlite`. Every note write goes through
//! [`upsert_note`]; every delete through [`remove_note`] / [`remove_path_recursive`].
use crate::error::AppResult;
use crate::model::*;
use crate::paths::parent_rel;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);

CREATE TABLE IF NOT EXISTS folders (
    path TEXT PRIMARY KEY,
    parent_path TEXT,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    parent_path TEXT,
    title TEXT NOT NULL,
    basename TEXT NOT NULL,
    mtime_ms INTEGER NOT NULL,
    size INTEGER NOT NULL,
    hash TEXT NOT NULL,
    is_db_record INTEGER NOT NULL DEFAULT 0,
    db_path TEXT
);
CREATE INDEX IF NOT EXISTS idx_notes_basename ON notes(basename);
CREATE INDEX IF NOT EXISTS idx_notes_parent ON notes(parent_path);
CREATE INDEX IF NOT EXISTS idx_notes_db ON notes(db_path);

CREATE TABLE IF NOT EXISTS properties (
    note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value_text TEXT,
    value_kind TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_props_note ON properties(note_id);
CREATE INDEX IF NOT EXISTS idx_props_key ON properties(key);

CREATE TABLE IF NOT EXISTS links (
    id INTEGER PRIMARY KEY,
    src_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    target_base TEXT NOT NULL,
    heading TEXT,
    block TEXT,
    alias TEXT,
    is_embed INTEGER NOT NULL DEFAULT 0,
    dest_id INTEGER,
    resolved INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_links_src ON links(src_id);
CREATE INDEX IF NOT EXISTS idx_links_dest ON links(dest_id);
CREATE INDEX IF NOT EXISTS idx_links_base ON links(target_base);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY,
    full TEXT NOT NULL UNIQUE,
    parent TEXT
);
CREATE TABLE IF NOT EXISTS note_tags (
    note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (note_id, tag_id)
);
CREATE INDEX IF NOT EXISTS idx_note_tags_tag ON note_tags(tag_id);

CREATE TABLE IF NOT EXISTS databases (
    path TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    schema_json TEXT NOT NULL,
    views_json TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
    path UNINDEXED, title, body, tags, tokenize = 'unicode61'
);
"#;

pub fn init_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(SCHEMA)?;
    conn.execute(
        "INSERT OR IGNORE INTO meta(key,value) VALUES('schema_version','1')",
        [],
    )?;
    Ok(())
}

/// Wipe all derived tables (disk is the source of truth) before a full rebuild.
pub fn clear_index(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "DELETE FROM properties; DELETE FROM links; DELETE FROM note_tags;
         DELETE FROM tags; DELETE FROM notes; DELETE FROM folders;
         DELETE FROM databases; DELETE FROM notes_fts;",
    )?;
    Ok(())
}

pub fn count_notes(conn: &Connection) -> AppResult<usize> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))?;
    Ok(n as usize)
}

// --- property <-> json -----------------------------------------------------

fn json_to_prop(v: &Value) -> (Option<String>, &'static str) {
    match v {
        Value::Null => (None, "null"),
        Value::Bool(b) => (Some(b.to_string()), "bool"),
        Value::Number(n) => (Some(n.to_string()), "number"),
        Value::String(s) => (Some(s.clone()), "string"),
        Value::Array(_) => (Some(v.to_string()), "list"),
        Value::Object(_) => (Some(v.to_string()), "object"),
    }
}

fn prop_to_json(text: Option<String>, kind: &str) -> Value {
    match (kind, text) {
        (_, None) | ("null", _) => Value::Null,
        ("bool", Some(t)) => Value::Bool(t == "true"),
        ("number", Some(t)) => t
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::String(t)),
        ("list" | "object", Some(t)) => serde_json::from_str(&t).unwrap_or(Value::String(t)),
        (_, Some(t)) => Value::String(t),
    }
}

// --- tags ------------------------------------------------------------------

/// Insert a hierarchical tag and all its ancestors; return the leaf tag id.
fn upsert_tag(conn: &Connection, full: &str) -> AppResult<i64> {
    let parts: Vec<&str> = full.split('/').collect();
    let mut last_id = 0i64;
    for i in 0..parts.len() {
        let cur = parts[..=i].join("/");
        let parent = if i == 0 {
            None
        } else {
            Some(parts[..i].join("/"))
        };
        conn.execute(
            "INSERT OR IGNORE INTO tags(full,parent) VALUES(?1,?2)",
            params![cur, parent],
        )?;
        last_id = conn.query_row("SELECT id FROM tags WHERE full=?1", [&cur], |r| r.get(0))?;
    }
    Ok(last_id)
}

// --- link resolution -------------------------------------------------------

/// Resolve all links pointing at the given basenames against current notes.
fn resolve_links_for_bases(conn: &Connection, bases: &HashSet<String>) -> AppResult<()> {
    for base in bases {
        conn.execute(
            "UPDATE links SET dest_id =
                (SELECT id FROM notes WHERE basename = ?1 ORDER BY length(path), path LIMIT 1)
             WHERE target_base = ?1",
            [base],
        )?;
        conn.execute(
            "UPDATE links SET resolved = (dest_id IS NOT NULL) WHERE target_base = ?1",
            [base],
        )?;
    }
    Ok(())
}

fn resolve_all_links(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "UPDATE links SET dest_id =
            (SELECT id FROM notes WHERE basename = links.target_base ORDER BY length(path), path LIMIT 1)",
        [],
    )?;
    conn.execute("UPDATE links SET resolved = (dest_id IS NOT NULL)", [])?;
    Ok(())
}

// --- note upsert / remove --------------------------------------------------

pub fn upsert_note(
    conn: &Connection,
    rel: &str,
    mtime_ms: i64,
    size: i64,
    content: &str,
) -> AppResult<NoteMeta> {
    let parsed = nexus_vault::index_note(rel, content);
    let hash = nexus_vault::hash_hex(content.as_bytes());
    let parent = parent_rel(rel);

    conn.execute(
        "INSERT INTO notes(path,parent_path,title,basename,mtime_ms,size,hash)
         VALUES(?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(path) DO UPDATE SET
            parent_path=excluded.parent_path, title=excluded.title, basename=excluded.basename,
            mtime_ms=excluded.mtime_ms, size=excluded.size, hash=excluded.hash",
        params![rel, parent, parsed.title, parsed.basename, mtime_ms, size, hash],
    )?;
    let note_id: i64 = conn.query_row("SELECT id FROM notes WHERE path=?1", [rel], |r| r.get(0))?;

    conn.execute("DELETE FROM properties WHERE note_id=?1", [note_id])?;
    conn.execute("DELETE FROM links WHERE src_id=?1", [note_id])?;
    conn.execute("DELETE FROM note_tags WHERE note_id=?1", [note_id])?;

    if let Some(obj) = parsed.frontmatter.as_object() {
        for (k, v) in obj {
            let (text, kind) = json_to_prop(v);
            conn.execute(
                "INSERT INTO properties(note_id,key,value_text,value_kind) VALUES(?1,?2,?3,?4)",
                params![note_id, k, text, kind],
            )?;
        }
    }

    let mut bases: HashSet<String> = HashSet::new();
    bases.insert(parsed.basename.clone());
    for t in &parsed.tags {
        let tid = upsert_tag(conn, t)?;
        conn.execute(
            "INSERT OR IGNORE INTO note_tags(note_id,tag_id) VALUES(?1,?2)",
            params![note_id, tid],
        )?;
    }
    for wl in &parsed.wikilinks {
        conn.execute(
            "INSERT INTO links(src_id,target_base,heading,block,alias,is_embed)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                note_id,
                wl.target_base,
                wl.heading,
                wl.block,
                wl.alias,
                wl.is_embed as i64
            ],
        )?;
        bases.insert(wl.target_base.clone());
    }

    conn.execute("DELETE FROM notes_fts WHERE path=?1", [rel])?;
    conn.execute(
        "INSERT INTO notes_fts(path,title,body,tags) VALUES(?1,?2,?3,?4)",
        params![rel, parsed.title, parsed.plain_text, parsed.tags.join(" ")],
    )?;

    resolve_links_for_bases(conn, &bases)?;

    Ok(NoteMeta {
        path: rel.to_string(),
        title: parsed.title,
        mtime_ms,
        hash,
        size,
    })
}

pub fn remove_note(conn: &Connection, rel: &str) -> AppResult<()> {
    let base = nexus_vault::basename_no_ext(rel);
    conn.execute("DELETE FROM notes WHERE path=?1", [rel])?;
    conn.execute("DELETE FROM notes_fts WHERE path=?1", [rel])?;
    let mut s = HashSet::new();
    s.insert(base);
    resolve_links_for_bases(conn, &s)?;
    Ok(())
}

pub fn upsert_folder(conn: &Connection, rel: &str) -> AppResult<()> {
    if rel.is_empty() {
        return Ok(());
    }
    let name = rel.rsplit('/').next().unwrap_or(rel).to_string();
    conn.execute(
        "INSERT OR REPLACE INTO folders(path,parent_path,name) VALUES(?1,?2,?3)",
        params![rel, parent_rel(rel), name],
    )?;
    Ok(())
}

/// Remove a path that may be a note or a folder subtree.
pub fn remove_path_recursive(conn: &Connection, rel: &str) -> AppResult<()> {
    let like = format!("{rel}/%");
    conn.execute(
        "DELETE FROM notes WHERE path=?1 OR path LIKE ?2",
        params![rel, like],
    )?;
    conn.execute(
        "DELETE FROM notes_fts WHERE path=?1 OR path LIKE ?2",
        params![rel, like],
    )?;
    conn.execute(
        "DELETE FROM folders WHERE path=?1 OR path LIKE ?2",
        params![rel, like],
    )?;
    conn.execute(
        "DELETE FROM databases WHERE path=?1 OR path LIKE ?2",
        params![rel, like],
    )?;
    resolve_all_links(conn)?;
    Ok(())
}

/// Register/refresh a Notion-style database from a `.nexusdb.json` marker.
pub fn register_database(conn: &Connection, folder_rel: &str, marker_json: &str) -> AppResult<()> {
    let val: Value = serde_json::from_str(marker_json).unwrap_or_else(|_| json!({}));
    let name = val
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            folder_rel
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or("Database")
                .to_string()
        });
    let columns = val.get("columns").cloned().unwrap_or_else(|| json!([]));
    let views = val
        .get("views")
        .cloned()
        .unwrap_or_else(|| json!([{"type":"table","name":"All"}]));
    conn.execute(
        "INSERT OR REPLACE INTO databases(path,name,schema_json,views_json) VALUES(?1,?2,?3,?4)",
        params![folder_rel, name, columns.to_string(), views.to_string()],
    )?;
    Ok(())
}

/// Recompute which notes belong to a database (parent folder is registered).
pub fn refresh_db_membership(conn: &Connection) -> AppResult<()> {
    conn.execute("UPDATE notes SET is_db_record=0, db_path=NULL", [])?;
    conn.execute(
        "UPDATE notes SET is_db_record=1, db_path=parent_path
         WHERE parent_path IN (SELECT path FROM databases)",
        [],
    )?;
    Ok(())
}

// --- tree ------------------------------------------------------------------

struct Stub {
    path: String,
    name: String,
    is_dir: bool,
}

pub fn get_tree(conn: &Connection, vault_name: &str) -> AppResult<TreeNode> {
    let mut by_parent: HashMap<String, Vec<Stub>> = HashMap::new();
    {
        let mut st = conn.prepare("SELECT path, COALESCE(parent_path,''), name FROM folders")?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(1)?,
                Stub {
                    path: r.get(0)?,
                    name: r.get(2)?,
                    is_dir: true,
                },
            ))
        })?;
        for row in rows {
            let (parent, s) = row?;
            by_parent.entry(parent).or_default().push(s);
        }
    }
    {
        let mut st = conn.prepare(
            "SELECT path, COALESCE(parent_path,''), COALESCE(NULLIF(title,''),basename) FROM notes",
        )?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(1)?,
                Stub {
                    path: r.get(0)?,
                    name: r.get(2)?,
                    is_dir: false,
                },
            ))
        })?;
        for row in rows {
            let (parent, s) = row?;
            by_parent.entry(parent).or_default().push(s);
        }
    }

    fn build(name: &str, path: &str, is_dir: bool, by_parent: &HashMap<String, Vec<Stub>>) -> TreeNode {
        let mut children = Vec::new();
        if is_dir {
            if let Some(list) = by_parent.get(path) {
                let mut sorted: Vec<&Stub> = list.iter().collect();
                sorted.sort_by(|a, b| {
                    b.is_dir
                        .cmp(&a.is_dir)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
                for s in sorted {
                    children.push(build(&s.name, &s.path, s.is_dir, by_parent));
                }
            }
        }
        TreeNode {
            path: path.to_string(),
            name: name.to_string(),
            is_dir,
            children,
        }
    }

    Ok(build(vault_name, "", true, &by_parent))
}

// --- backlinks / outline / tags --------------------------------------------

pub fn get_backlinks(conn: &Connection, path: &str) -> AppResult<Vec<Backlink>> {
    let mut st = conn.prepare(
        "SELECT src.path, COALESCE(NULLIF(src.title,''),src.basename), MAX(l.alias)
         FROM links l JOIN notes src ON src.id = l.src_id
         WHERE l.resolved=1 AND l.dest_id = (SELECT id FROM notes WHERE path=?1)
         GROUP BY src.id ORDER BY src.title",
    )?;
    let rows = st.query_map([path], |r| {
        Ok(Backlink {
            path: r.get(0)?,
            title: r.get(1)?,
            alias: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_tags(conn: &Connection) -> AppResult<Vec<TagCount>> {
    let mut st = conn.prepare(
        "SELECT t.full, COUNT(nt.note_id) FROM tags t
         LEFT JOIN note_tags nt ON nt.tag_id=t.id GROUP BY t.id ORDER BY t.full",
    )?;
    let rows = st.query_map([], |r| {
        Ok(TagCount {
            full: r.get(0)?,
            count: r.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn notes_with_tag(conn: &Connection, tag: &str, include_subtags: bool) -> AppResult<Vec<SearchHit>> {
    let sub = format!("{tag}/%");
    let mut st = conn.prepare(
        "SELECT DISTINCT n.path, COALESCE(NULLIF(n.title,''),n.basename)
         FROM notes n JOIN note_tags nt ON nt.note_id=n.id JOIN tags t ON t.id=nt.tag_id
         WHERE t.full = ?1 OR (?2 = 1 AND t.full LIKE ?3) ORDER BY n.title",
    )?;
    let rows = st.query_map(params![tag, include_subtags as i64, sub], |r| {
        Ok(SearchHit {
            path: r.get(0)?,
            title: r.get(1)?,
            snippet: String::new(),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

// --- search ----------------------------------------------------------------

fn fts_query(q: &str) -> String {
    let has_ops = q.contains('"')
        || q.contains('*')
        || q.contains(':')
        || q.split_whitespace().any(|w| matches!(w, "AND" | "OR" | "NOT"));
    if has_ops {
        return q.to_string();
    }
    q.split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        })
        .filter(|s| !s.is_empty())
        .map(|s| format!("{s}*"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn search(conn: &Connection, query: &str, limit: i64) -> AppResult<Vec<SearchHit>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(vec![]);
    }
    let fts = fts_query(trimmed);
    if !fts.is_empty() {
        let attempt = (|| -> rusqlite::Result<Vec<SearchHit>> {
            let mut st = conn.prepare(
                "SELECT path, title, snippet(notes_fts,2,'<mark>','</mark>','…',12)
                 FROM notes_fts WHERE notes_fts MATCH ?1 ORDER BY rank LIMIT ?2",
            )?;
            let rows = st.query_map(params![fts, limit], |r| {
                Ok(SearchHit {
                    path: r.get(0)?,
                    title: r.get(1)?,
                    snippet: r.get(2)?,
                })
            })?;
            rows.collect()
        })();
        if let Ok(hits) = attempt {
            return Ok(hits);
        }
    }
    // Fallback: LIKE over titles (handles FTS syntax errors / empty token sets).
    let like = format!("%{trimmed}%");
    let mut st = conn.prepare(
        "SELECT path, COALESCE(NULLIF(title,''),basename), '' FROM notes
         WHERE title LIKE ?1 OR basename LIKE ?1 ORDER BY title LIMIT ?2",
    )?;
    let rows = st.query_map(params![like, limit], |r| {
        Ok(SearchHit {
            path: r.get(0)?,
            title: r.get(1)?,
            snippet: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn search_suggest(conn: &Connection, prefix: &str) -> AppResult<Vec<Suggestion>> {
    let p = format!("{}%", prefix.trim());
    let mut out = Vec::new();
    {
        let mut st = conn.prepare(
            "SELECT COALESCE(NULLIF(title,''),basename), path FROM notes
             WHERE title LIKE ?1 OR basename LIKE ?1 ORDER BY length(title) LIMIT 8",
        )?;
        let rows = st.query_map([&p], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (label, path) = row?;
            out.push(Suggestion {
                kind: "note".into(),
                label,
                path: Some(path),
            });
        }
    }
    {
        let mut st = conn.prepare("SELECT full FROM tags WHERE full LIKE ?1 ORDER BY full LIMIT 5")?;
        let rows = st.query_map([&p], |r| r.get::<_, String>(0))?;
        for row in rows {
            out.push(Suggestion {
                kind: "tag".into(),
                label: row?,
                path: None,
            });
        }
    }
    Ok(out)
}

// --- graph -----------------------------------------------------------------

pub fn graph_global(conn: &Connection) -> AppResult<GraphData> {
    let mut nodes = Vec::new();
    {
        let mut st = conn.prepare(
            "SELECT path, COALESCE(NULLIF(title,''),basename) FROM notes",
        )?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (path, title) = row?;
            let group = match path.split_once('/') {
                Some((top, _)) => top.to_string(),
                None => "root".to_string(),
            };
            nodes.push(GraphNode {
                id: path,
                label: title,
                val: 1.0,
                group,
            });
        }
    }
    let mut links = Vec::new();
    {
        let mut st = conn.prepare(
            "SELECT src.path, tgt.path FROM links l
             JOIN notes src ON src.id=l.src_id JOIN notes tgt ON tgt.id=l.dest_id
             WHERE l.resolved=1 AND src.path <> tgt.path",
        )?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (source, target) = row?;
            links.push(GraphEdge { source, target });
        }
    }
    let mut deg: HashMap<String, f64> = HashMap::new();
    for e in &links {
        *deg.entry(e.source.clone()).or_default() += 1.0;
        *deg.entry(e.target.clone()).or_default() += 1.0;
    }
    for n in &mut nodes {
        n.val = 1.0 + deg.get(&n.id).copied().unwrap_or(0.0);
    }
    Ok(GraphData { nodes, links })
}

pub fn graph_local(conn: &Connection, path: &str, depth: u8) -> AppResult<GraphData> {
    let g = graph_global(conn)?;
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &g.links {
        adj.entry(e.source.as_str()).or_default().push(&e.target);
        adj.entry(e.target.as_str()).or_default().push(&e.source);
    }
    let mut keep: HashSet<String> = HashSet::new();
    keep.insert(path.to_string());
    let mut frontier = vec![path.to_string()];
    for _ in 0..depth.max(1) {
        let mut next = Vec::new();
        for p in &frontier {
            if let Some(ns) = adj.get(p.as_str()) {
                for n in ns {
                    if keep.insert(n.to_string()) {
                        next.push(n.to_string());
                    }
                }
            }
        }
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }
    Ok(GraphData {
        nodes: g.nodes.into_iter().filter(|n| keep.contains(&n.id)).collect(),
        links: g
            .links
            .into_iter()
            .filter(|e| keep.contains(&e.source) && keep.contains(&e.target))
            .collect(),
    })
}

// --- databases -------------------------------------------------------------

pub fn list_databases(conn: &Connection) -> AppResult<Vec<DbInfo>> {
    let mut st = conn.prepare(
        "SELECT d.path, d.name, (SELECT COUNT(*) FROM notes n WHERE n.db_path=d.path)
         FROM databases d ORDER BY d.name",
    )?;
    let rows = st.query_map([], |r| {
        Ok(DbInfo {
            path: r.get(0)?,
            name: r.get(1)?,
            count: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_database(conn: &Connection, path: &str) -> AppResult<DbSchema> {
    let (name, schema_json, views_json): (String, String, String) = conn.query_row(
        "SELECT name, schema_json, views_json FROM databases WHERE path=?1",
        [path],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok(DbSchema {
        path: path.to_string(),
        name,
        columns: serde_json::from_str(&schema_json).unwrap_or_default(),
        views: serde_json::from_str(&views_json).unwrap_or_default(),
    })
}

pub fn query_database(conn: &Connection, path: &str) -> AppResult<DbResult> {
    let schema = get_database(conn, path)?;
    let recs: Vec<(i64, String, String)> = {
        let mut st = conn.prepare(
            "SELECT id, path, COALESCE(NULLIF(title,''),basename) FROM notes
             WHERE db_path=?1 ORDER BY title",
        )?;
        let v = st
            .query_map([path], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        v
    };
    let mut rows = Vec::new();
    for (id, p, title) in recs {
        let mut fields = serde_json::Map::new();
        let mut ps = conn.prepare("SELECT key,value_text,value_kind FROM properties WHERE note_id=?1")?;
        let prows = ps.query_map([id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        for row in prows {
            let (k, text, kind) = row?;
            fields.insert(k, prop_to_json(text, &kind));
        }
        rows.push(DbRow {
            path: p,
            title,
            fields: Value::Object(fields),
        });
    }
    Ok(DbResult { schema, rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        init_schema(&c).unwrap();
        c
    }

    #[test]
    fn backlinks_search_and_graph() {
        let c = db();
        upsert_note(&c, "A.md", 0, 16, "# A\nlinks to [[B]]").unwrap();
        upsert_note(&c, "B.md", 0, 24, "# B\nall about widgets").unwrap();

        let bl = get_backlinks(&c, "B.md").unwrap();
        assert_eq!(bl.len(), 1);
        assert_eq!(bl[0].path, "A.md");

        let hits = search(&c, "widgets", 10).unwrap();
        assert!(hits.iter().any(|h| h.path == "B.md"));

        let g = graph_global(&c).unwrap();
        assert_eq!(g.links.len(), 1);
        assert_eq!(g.nodes.len(), 2);
    }

    #[test]
    fn hierarchical_tags_indexed() {
        let c = db();
        upsert_note(&c, "N.md", 0, 5, "#project/active hello").unwrap();
        let tags = list_tags(&c).unwrap();
        assert!(tags.iter().any(|t| t.full == "project/active"));
        assert!(tags.iter().any(|t| t.full == "project")); // ancestor auto-created
    }

    #[test]
    fn removing_target_unresolves_backlinks() {
        let c = db();
        upsert_note(&c, "A.md", 0, 5, "[[B]]").unwrap();
        upsert_note(&c, "B.md", 0, 5, "b").unwrap();
        assert_eq!(get_backlinks(&c, "B.md").unwrap().len(), 1);
        remove_note(&c, "B.md").unwrap();
        assert_eq!(get_backlinks(&c, "B.md").unwrap().len(), 0);
    }

    #[test]
    fn database_records_expose_frontmatter() {
        let c = db();
        register_database(&c, "Tasks", r#"{"name":"Tasks","columns":[{"key":"status","type":"select","options":["todo","done"]}],"views":[]}"#).unwrap();
        upsert_note(&c, "Tasks/T1.md", 0, 30, "---\nstatus: done\n---\nbody").unwrap();
        refresh_db_membership(&c).unwrap();
        let res = query_database(&c, "Tasks").unwrap();
        assert_eq!(res.rows.len(), 1);
        assert_eq!(res.rows[0].fields["status"], serde_json::json!("done"));
    }
}
