use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum VizError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported graph export version: {0}")]
    UnsupportedVersion(String),
    #[error("projection produced no nodes")]
    EmptyScene,
}
