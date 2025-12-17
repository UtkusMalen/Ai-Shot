//! Error types for the ai-shot-core library.
//!
//! This module provides granular error variants for different failure modes,
//! enabling precise error handling and user-friendly error messages.

use thiserror::Error;

/// Errors that can occur within the ai-shot-core library.
///
/// Each variant represents a specific failure mode with contextual information
/// to help diagnose and handle errors appropriately.
#[derive(Error, Debug)]
pub enum AppError {
    /// Configuration-related errors (missing keys, invalid values).
    #[error("Configuration error: {0}")]
    Config(String),

    /// A required environment variable was not found.
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),

    /// Screen capture operation failed.
    #[error("Screen capture failed: {0}")]
    ScreenCapture(String),

    /// Requested screen/monitor index was not found.
    #[error("Screen not found: index {0}")]
    ScreenNotFound(usize),

    /// Image processing or encoding failed.
    #[error("Image processing failed: {0}")]
    ImageProcessing(String),

    /// The selection area is empty or has zero dimensions.
    #[error("Selection area is empty or invalid")]
    EmptySelection,

    /// General Gemini API error.
    #[error("Gemini API error: {0}")]
    GeminiApi(String),

    /// Rate limited by the Gemini API.
    #[error("Rate limited by Gemini API, please retry later")]
    RateLimited,

    /// UI-related errors (rendering, window management).
    #[error("UI error: {0}")]
    Ui(String),

    /// Standard I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// An unclassified error.
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl AppError {
    /// Creates a configuration error with the given message.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Creates a screen capture error with the given message.
    pub fn capture(msg: impl Into<String>) -> Self {
        Self::ScreenCapture(msg.into())
    }

    /// Creates an image processing error with the given message.
    pub fn image(msg: impl Into<String>) -> Self {
        Self::ImageProcessing(msg.into())
    }

    /// Creates a Gemini API error with the given message.
    pub fn gemini(msg: impl Into<String>) -> Self {
        Self::GeminiApi(msg.into())
    }

    /// Creates a UI error with the given message.
    pub fn ui(msg: impl Into<String>) -> Self {
        Self::Ui(msg.into())
    }
}

/// A convenient alias for Result with [`AppError`].
pub type Result<T> = std::result::Result<T, AppError>;