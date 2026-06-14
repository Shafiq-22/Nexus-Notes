//! Pure, GUI-free vault logic for Nexus Notes.
//!
//! This crate has **no dependency on Tauri or the filesystem**, so all of the
//! tricky parsing — frontmatter, wikilinks, tags, headings, hashing — can be
//! unit-tested headlessly with `cargo test -p nexus-vault`. The Tauri backend
//! treats Markdown on disk as the source of truth and calls [`index_note`] to
//! derive everything it needs for the SQLite/search indexes.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeSet;

use pulldown_cmark::{Event, Options, Parser};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A note split into its YAML frontmatter (as JSON) and the Markdown body.
#[derive(Debug, Clone)]
pub struct ParsedNote {
    pub frontmatter: serde_json::Value,
    pub body: String,
}

/// A `[[wikilink]]` (or `![[embed]]`) parsed into its components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WikiLink {
    /// The inner text exactly as written, e.g. `Target#Heading|alias`.
    pub raw: String,
    /// Normalized lookup key (the part before `#`), e.g. `Target`.
    pub target_base: String,
    pub heading: Option<String>,
    pub block: Option<String>,
    pub alias: Option<String>,
    pub is_embed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    /// 0-based line index within the body.
    pub line: usize,
}

/// Everything the backend needs to index a single note.
#[derive(Debug, Clone, Serialize)]
pub struct IndexedNote {
    pub title: String,
    pub basename: String,
    pub frontmatter: serde_json::Value,
    pub plain_text: String,
    pub wikilinks: Vec<WikiLink>,
    pub tags: Vec<String>,
    pub headings: Vec<Heading>,
}

// ---------------------------------------------------------------------------
// Regexes (compiled once)
// ---------------------------------------------------------------------------

static RE_WIKILINK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(!?)\[\[([^\[\]\n]+)\]\]").unwrap());
static RE_TAG: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:^|[\s(>\[])#([A-Za-z][A-Za-z0-9_/\-]*)").unwrap());
static RE_HEADING_LINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(#{1,6})\s+(.+?)\s*#*\s*$").unwrap());
static RE_FENCE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?ms)^[ \t]*```.*?```[ \t]*$").unwrap());
static RE_INLINE_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"`[^`\n]*`").unwrap());

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// blake3 content hash as a lowercase hex string. Used for change detection and
/// echo-suppression of the app's own writes.
pub fn hash_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

// ---------------------------------------------------------------------------
// Frontmatter
// ---------------------------------------------------------------------------

/// Split leading `---` YAML frontmatter from the body. Frontmatter must be at
/// the very start of the file. Invalid/absent frontmatter yields an empty object
/// and the full content as the body.
pub fn split_frontmatter(content: &str) -> ParsedNote {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let empty = || serde_json::Value::Object(serde_json::Map::new());

    let mut lines = content.split('\n');
    let first = lines.next();
    if first.map(|l| l.trim_end_matches('\r')) == Some("---") {
        let mut yaml = String::new();
        let mut closed = false;
        let mut body_lines: Vec<&str> = Vec::new();
        for line in lines {
            let l = line.trim_end_matches('\r');
            if !closed && (l == "---" || l == "...") {
                closed = true;
                continue;
            }
            if closed {
                body_lines.push(line);
            } else {
                yaml.push_str(l);
                yaml.push('\n');
            }
        }
        if closed {
            let fm = serde_yaml::from_str::<serde_yaml::Value>(&yaml)
                .ok()
                .and_then(|y| serde_json::to_value(y).ok())
                .filter(|v| v.is_object())
                .unwrap_or_else(empty);
            return ParsedNote {
                frontmatter: fm,
                body: body_lines.join("\n").trim_start_matches('\n').to_string(),
            };
        }
    }
    ParsedNote {
        frontmatter: empty(),
        body: content.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

/// Remove fenced and inline code so links/tags inside code aren't extracted.
fn strip_code(body: &str) -> String {
    let no_fence = RE_FENCE.replace_all(body, " ");
    RE_INLINE_CODE.replace_all(&no_fence, " ").to_string()
}

/// Parse all `[[wikilinks]]` and `![[embeds]]` out of a body.
pub fn extract_wikilinks(body: &str) -> Vec<WikiLink> {
    let cleaned = strip_code(body);
    let mut out = Vec::new();
    for cap in RE_WIKILINK.captures_iter(&cleaned) {
        let is_embed = &cap[1] == "!";
        let inner = cap[2].trim().to_string();

        let (link_part, alias) = match inner.split_once('|') {
            Some((l, a)) => (l.trim().to_string(), Some(a.trim().to_string())),
            None => (inner.clone(), None),
        };
        let (base, sub) = match link_part.split_once('#') {
            Some((b, s)) => (b.trim().to_string(), Some(s.trim().to_string())),
            None => (link_part.trim().to_string(), None),
        };
        let (heading, block) = match sub {
            Some(s) if s.starts_with('^') => (None, Some(s[1..].trim().to_string())),
            Some(s) if !s.is_empty() => (Some(s), None),
            _ => (None, None),
        };
        if base.is_empty() && heading.is_none() && block.is_none() {
            continue;
        }
        out.push(WikiLink {
            raw: inner,
            target_base: base,
            heading,
            block,
            alias,
            is_embed,
        });
    }
    out
}

fn collect_fm_tags(val: &serde_json::Value, set: &mut BTreeSet<String>) {
    match val {
        serde_json::Value::String(s) => {
            for part in s.split([',', ' ']) {
                let p = part.trim().trim_start_matches('#');
                if !p.is_empty() {
                    set.insert(p.to_string());
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                if let Some(s) = v.as_str() {
                    let p = s.trim().trim_start_matches('#');
                    if !p.is_empty() {
                        set.insert(p.to_string());
                    }
                }
            }
        }
        _ => {}
    }
}

/// Extract hierarchical tags from inline `#tag/sub` syntax and the frontmatter
/// `tags:` field. Pure-numeric `#123` (heading anchors) are ignored.
pub fn extract_tags(body: &str, frontmatter: &serde_json::Value) -> Vec<String> {
    let mut set = BTreeSet::new();
    let cleaned = strip_code(body);
    for cap in RE_TAG.captures_iter(&cleaned) {
        let t = cap[1].trim_end_matches('/').to_string();
        if !t.is_empty() {
            set.insert(t);
        }
    }
    if let Some(tags) = frontmatter.get("tags") {
        collect_fm_tags(tags, &mut set);
    }
    set.into_iter().collect()
}

/// Extract ATX headings (`#`..`######`), skipping fenced code blocks.
pub fn extract_headings(body: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(cap) = RE_HEADING_LINE.captures(line) {
            out.push(Heading {
                level: cap[1].len() as u8,
                text: cap[2].trim().to_string(),
                line: i,
            });
        }
    }
    out
}

/// Flatten Markdown to plain text for search snippets and FTS indexing.
pub fn to_plain_text(body: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(body, opts);
    let mut s = String::new();
    for ev in parser {
        match ev {
            Event::Text(t) | Event::Code(t) => {
                s.push_str(&t);
                s.push(' ');
            }
            Event::SoftBreak | Event::HardBreak => s.push(' '),
            _ => {}
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Convenience
// ---------------------------------------------------------------------------

/// File basename without the `.md` extension (used for wikilink resolution).
pub fn basename_no_ext(path: &str) -> String {
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    base.strip_suffix(".md").unwrap_or(base).to_string()
}

/// Resolve a note's display title: frontmatter `title` → first H1 → basename.
pub fn title_of(path: &str, frontmatter: &serde_json::Value, body: &str) -> String {
    if let Some(t) = frontmatter.get("title").and_then(|v| v.as_str()) {
        if !t.trim().is_empty() {
            return t.trim().to_string();
        }
    }
    for h in extract_headings(body) {
        if h.level == 1 {
            return h.text;
        }
    }
    basename_no_ext(path)
}

/// Compose a note's on-disk Markdown from its frontmatter (JSON) and body.
/// Deterministic: serde_json objects use sorted keys, so re-saving is byte-stable.
/// Used when writing notes and when DB cell edits mutate only the frontmatter.
pub fn compose_note(frontmatter: &serde_json::Value, body: &str) -> String {
    let has_fm = frontmatter
        .as_object()
        .map(|o| !o.is_empty())
        .unwrap_or(false);
    let body = body.trim_end_matches('\n');
    if !has_fm {
        return format!("{body}\n");
    }
    let yaml = serde_yaml::to_string(frontmatter).unwrap_or_default();
    format!("---\n{}---\n\n{}\n", yaml, body.trim_start_matches('\n'))
}

/// Parse a note end-to-end into everything the backend indexes.
pub fn index_note(path: &str, content: &str) -> IndexedNote {
    let parsed = split_frontmatter(content);
    let title = title_of(path, &parsed.frontmatter, &parsed.body);
    IndexedNote {
        title,
        basename: basename_no_ext(path),
        wikilinks: extract_wikilinks(&parsed.body),
        tags: extract_tags(&parsed.body, &parsed.frontmatter),
        headings: extract_headings(&parsed.body),
        plain_text: to_plain_text(&parsed.body),
        frontmatter: parsed.frontmatter,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_frontmatter() {
        let p = split_frontmatter("---\ntitle: Hello\ntags: [a, b]\n---\n\n# Body\ntext");
        assert_eq!(p.frontmatter["title"], "Hello");
        assert!(p.body.starts_with("# Body"));
    }

    #[test]
    fn no_frontmatter_is_all_body() {
        let p = split_frontmatter("# Just a note\nhi");
        assert!(p.frontmatter.as_object().unwrap().is_empty());
        assert_eq!(p.body, "# Just a note\nhi");
    }

    #[test]
    fn unterminated_frontmatter_is_body() {
        let p = split_frontmatter("---\ntitle: oops\nno close");
        assert!(p.frontmatter.as_object().unwrap().is_empty());
        assert!(p.body.contains("no close"));
    }

    #[test]
    fn parses_wikilinks_with_parts() {
        let links = extract_wikilinks("See [[Vision#Q3|the plan]] and ![[Budget]] and [[Plain]]");
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].target_base, "Vision");
        assert_eq!(links[0].heading.as_deref(), Some("Q3"));
        assert_eq!(links[0].alias.as_deref(), Some("the plan"));
        assert!(!links[0].is_embed);
        assert!(links[1].is_embed);
        assert_eq!(links[1].target_base, "Budget");
        assert_eq!(links[2].target_base, "Plain");
    }

    #[test]
    fn parses_block_reference() {
        let links = extract_wikilinks("ref [[Note#^abc123]]");
        assert_eq!(links[0].target_base, "Note");
        assert_eq!(links[0].block.as_deref(), Some("abc123"));
        assert!(links[0].heading.is_none());
    }

    #[test]
    fn ignores_links_in_code() {
        let links = extract_wikilinks("real [[Yes]]\n```\n[[No]]\n```\ninline `[[Nope]]`");
        let bases: Vec<_> = links.iter().map(|l| l.target_base.as_str()).collect();
        assert_eq!(bases, vec!["Yes"]);
    }

    #[test]
    fn extracts_tags_inline_and_frontmatter() {
        let p = split_frontmatter("---\ntags: [area/work]\n---\nHas #project/active and #idea here");
        let tags = extract_tags(&p.body, &p.frontmatter);
        assert!(tags.contains(&"project/active".to_string()));
        assert!(tags.contains(&"idea".to_string()));
        assert!(tags.contains(&"area/work".to_string()));
    }

    #[test]
    fn extracts_headings_skipping_fence() {
        let h = extract_headings("# Title\n```\n# not a heading\n```\n## Sub");
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].level, 1);
        assert_eq!(h[0].text, "Title");
        assert_eq!(h[1].level, 2);
    }

    #[test]
    fn title_resolution_order() {
        let none = serde_json::json!({});
        assert_eq!(title_of("dir/My Note.md", &none, "no heading"), "My Note");
        assert_eq!(title_of("dir/My Note.md", &none, "# H1 Wins"), "H1 Wins");
        let fm = serde_json::json!({"title": "FM Wins"});
        assert_eq!(title_of("dir/My Note.md", &fm, "# H1"), "FM Wins");
    }

    #[test]
    fn plain_text_strips_markdown() {
        let t = to_plain_text("# Heading\n\n**bold** and `code` and [link](http://x)");
        assert!(t.contains("Heading"));
        assert!(t.contains("bold"));
        assert!(t.contains("code"));
        assert!(!t.contains('*'));
    }

    #[test]
    fn hashing_is_stable() {
        assert_eq!(hash_hex(b"abc"), hash_hex(b"abc"));
        assert_ne!(hash_hex(b"abc"), hash_hex(b"abd"));
    }

    #[test]
    fn compose_round_trips() {
        let fm = serde_json::json!({"title": "X", "tags": ["a"]});
        let composed = compose_note(&fm, "# X\nbody");
        let parsed = split_frontmatter(&composed);
        assert_eq!(parsed.frontmatter["title"], "X");
        assert!(parsed.body.contains("body"));
        // empty frontmatter → no fence
        assert_eq!(compose_note(&serde_json::json!({}), "just body"), "just body\n");
        // deterministic
        assert_eq!(compose_note(&fm, "# X\nbody"), composed);
    }

    #[test]
    fn index_note_end_to_end() {
        let n = index_note(
            "notes/Roadmap.md",
            "---\ntitle: Roadmap\ntags: [planning]\n---\n# Roadmap\nLinks to [[Vision]] #project",
        );
        assert_eq!(n.title, "Roadmap");
        assert_eq!(n.basename, "Roadmap");
        assert_eq!(n.wikilinks.len(), 1);
        assert!(n.tags.contains(&"planning".to_string()));
        assert!(n.tags.contains(&"project".to_string()));
    }
}
