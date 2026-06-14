use crate::model::NoteMeta;
use serde::Serialize;

pub mod names {
    pub const FS_CREATED: &str = "fs:created";
    pub const FS_CHANGED: &str = "fs:changed";
    pub const FS_DELETED: &str = "fs:deleted";
    pub const INDEX_PROGRESS: &str = "index:progress";
    pub const INDEX_READY: &str = "index:ready";
    pub const SYNC_STATUS: &str = "sync:status";
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsEvent {
    pub path: String,
    /// "note" | "folder"
    pub kind: String,
    pub meta: Option<NoteMeta>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub done: usize,
    pub total: usize,
    pub phase: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexReady {
    pub note_count: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    /// "idle" | "watching" | "reconciling"
    pub state: String,
}
