use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Screen capture failed: {0}")]
    ScreenCapture(String),

    #[error("Image processing failed: {0}")]
    ImageProcessing(String),

    #[error("Gemini API error: {0}")]
    GeminiApi(String),

    #[error("UI error: {0}")]
    Ui(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// A convenient alias for Result
pub type Result<T> = std::result::Result<T, AppError>;