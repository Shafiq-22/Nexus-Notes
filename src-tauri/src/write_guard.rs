//! Echo-suppression registry that prevents the write→watch feedback loop.
//!
//! Before the app writes/renames/deletes a path it calls [`WriteGuard::arm`].
//! When the filesystem watcher later fires for that path, [`WriteGuard::should_suppress`]
//! returns `true` if the event is just the echo of our own write.
//!
//! Strategy (see docs/04-filesystem-sync.md): **content hash is the primary
//! test** — if the bytes on disk equal what we wrote, it's our echo regardless
//! of timing (robust to coalesced/delayed inotify events). A short **time
//! window** is the safety net for ops that have no hash (deletes, folders,
//! renames). A hash match consumes the entry so a later *genuine* external save
//! of identical-looking content is still processed.
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const IGNORE_WINDOW: Duration = Duration::from_millis(2500);

struct Entry {
    hash: Option<String>,
    expires: Instant,
}

#[derive(Default)]
pub struct WriteGuard {
    inner: Mutex<HashMap<PathBuf, Entry>>,
}

impl WriteGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an imminent app write. `hash` is the blake3 hex of the bytes we
    /// are about to write, or `None` for deletes / directory operations.
    pub fn arm(&self, path: &Path, hash: Option<String>) {
        self.inner.lock().insert(
            path.to_path_buf(),
            Entry {
                hash,
                expires: Instant::now() + IGNORE_WINDOW,
            },
        );
    }

    /// Should this watcher event be ignored as our own echo?
    pub fn should_suppress(&self, path: &Path, observed_hash: Option<&str>) -> bool {
        let mut map = self.inner.lock();
        let now = Instant::now();
        map.retain(|_, e| e.expires > now);

        match map.get(path) {
            None => false,
            Some(entry) => match (entry.hash.as_deref(), observed_hash) {
                // Same bytes we wrote → our echo. Consume so a later real edit is seen.
                (Some(h), Some(o)) if h == o => {
                    map.remove(path);
                    true
                }
                // We recorded a hash but disk differs → a genuine external edit.
                (Some(_), Some(_)) => false,
                // No hash on record (delete/folder/rename) → time-window suppress.
                _ => {
                    map.remove(path);
                    true
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn matching_hash_is_echo_then_consumed() {
        let g = WriteGuard::new();
        let p = Path::new("/v/a.md");
        g.arm(p, Some("h1".into()));
        assert!(g.should_suppress(p, Some("h1"))); // our own write → suppress
        assert!(!g.should_suppress(p, Some("h1"))); // consumed → next is processed
    }

    #[test]
    fn different_hash_is_genuine_edit() {
        let g = WriteGuard::new();
        let p = Path::new("/v/a.md");
        g.arm(p, Some("h1".into()));
        assert!(!g.should_suppress(p, Some("h2")));
    }

    #[test]
    fn no_hash_ops_suppressed_in_window() {
        let g = WriteGuard::new();
        let p = Path::new("/v/dir");
        g.arm(p, None);
        assert!(g.should_suppress(p, None));
    }

    #[test]
    fn unarmed_paths_pass_through() {
        let g = WriteGuard::new();
        assert!(!g.should_suppress(Path::new("/v/x.md"), Some("h")));
    }
}
