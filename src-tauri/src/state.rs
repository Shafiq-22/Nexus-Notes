use crate::vault::Vault;
use parking_lot::Mutex;

/// Global app state managed by Tauri. The vault sits behind a Mutex so it can be
/// swapped when switching vaults; commands clone out the `Arc`s they need and
/// release the lock before doing IO.
#[derive(Default)]
pub struct AppState {
    pub vault: Mutex<Option<Vault>>,
}
