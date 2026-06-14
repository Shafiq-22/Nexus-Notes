use serde::{Serialize, Serializer};

/// All command errors. Serializes to a plain string for the frontend.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("no vault is open")]
    NoVault,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("invalid path")]
    InvalidPath,
    #[error("a file or folder already exists at that path")]
    AlreadyExists,
    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Other(format!("json: {e}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;
