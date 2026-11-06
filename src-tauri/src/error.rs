use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("download error: {0}")]
    Download(String),
    #[error("cancelled")]
    Cancelled,
    #[error("{0}")]
    Other(String),
}

impl From<ort::Error> for AppError {
    fn from(e: ort::Error) -> Self {
        AppError::Other(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Download(e.to_string())
    }
}

/// Stringified at the command boundary — `ort::Error` is not `Send + Sync`,
/// so concrete error types must not cross into command signatures.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CmdError {
    pub message: String,
    pub cancelled: bool,
}

impl From<AppError> for CmdError {
    fn from(e: AppError) -> Self {
        let cancelled = matches!(e, AppError::Cancelled);
        CmdError {
            message: e.to_string(),
            cancelled,
        }
    }
}
