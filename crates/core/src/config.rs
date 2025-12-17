//! Configuration management for ai-shot.
//!
//! This module handles loading configuration from environment variables
//! and `.env` files, with a builder pattern for flexible initialization.

use crate::error::{AppError, Result};
use std::env;

/// Application configuration containing API keys and model settings.
///
/// # Example
/// ```ignore
/// // Load from environment
/// let config = Config::load()?;
///
/// // Or use the builder
/// let config = Config::builder()
///     .with_model("gemini-2.5-pro")
///     .build()?;
/// ```
#[derive(Clone, Debug)]
pub struct Config {
    /// Gemini API key for authentication.
    pub gemini_api_key: String,
    /// Model name to use (e.g., "gemini-flash-latest").
    pub model_name: String,
}

/// Builder for [`Config`] with sensible defaults.
///
/// Allows overriding specific values while loading others from the environment.
#[derive(Default)]
pub struct ConfigBuilder {
    api_key: Option<String>,
    model_name: Option<String>,
}

impl ConfigBuilder {
    /// Sets a custom model name, overriding the environment variable.
    ///
    /// # Arguments
    /// * `model` - The model name (e.g., "gemini-2.5-pro")
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_name = Some(model.into());
        self
    }

    /// Sets the API key directly, overriding the environment variable.
    ///
    /// Useful for testing or when the key is obtained at runtime.
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Builds the configuration.
    ///
    /// Values not explicitly set are loaded from environment variables.
    /// The API key is required and will return an error if not found.
    ///
    /// # Errors
    /// Returns [`AppError`] if configuration validation fails.
    /// Note: `GEMINI_API_KEY` is no longer strictly required at build time
    /// and can be provided later via UI settings.
    pub fn build(self) -> Result<Config> {
        // Try explicit value first, then environment variable, then default to empty
        let api_key = self
            .api_key
            .or_else(|| env::var("GEMINI_API_KEY").ok())
            .unwrap_or_default();

        // Model has a sensible default
        let model_name = self
            .model_name
            .or_else(|| env::var("GEMINI_MODEL").ok())
            .unwrap_or_else(|| "gemini-flash-latest".to_string());

        Ok(Config {
            gemini_api_key: api_key,
            model_name,
        })
    }
}

impl Config {
    /// Creates a new [`ConfigBuilder`] for fluent configuration.
    ///
    /// # Example
    /// ```ignore
    /// let config = Config::builder()
    ///     .with_model("gemini-2.5-pro")
    ///     .build()?;
    /// ```
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Loads configuration from environment variables.
    ///
    /// This is a convenience method equivalent to `Config::builder().build()`.
    /// It expects `GEMINI_API_KEY` to be set in the environment or `.env` file.
    ///
    /// # Errors
    /// Returns error if configuration loading fails.
    pub fn load() -> Result<Self> {
        Self::builder().build()
    }

    /// Creates a config with a specific API key and default model.
    ///
    /// Useful for testing or programmatic initialization.
    pub fn with_key(api_key: impl Into<String>) -> Self {
        Self {
            gemini_api_key: api_key.into(),
            model_name: "gemini-flash-latest".to_string(),
        }
    }
}