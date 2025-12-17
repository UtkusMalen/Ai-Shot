//! User settings persistence and UI configuration.
//!
//! This module handles loading and saving user preferences,
//! including model selection, API keys, and feature toggles.

use crate::error::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Available Gemini models for selection in the UI.
pub const AVAILABLE_MODELS: &[&str] = &[
    "gemini-2.5-pro",
    "gemini-flash-latest",
    "gemini-flash-lite-latest",
];

/// User-configurable settings persisted between sessions.
///
/// Settings are stored as JSON in the user's config directory
/// (e.g., `~/.config/ai-shot/settings.json` on Linux).
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Selected Gemini model name.
    pub model: String,
    /// System prompt prepended to all requests.
    pub system_prompt: String,
    /// Enable "thinking" mode (Gemini 2.0+ models).
    pub thinking_enabled: bool,
    /// Enable Google Search grounding for responses.
    pub google_search: bool,
    /// API key override (takes precedence over environment).
    #[serde(default)]
    pub api_key: String,
}

impl Settings {
    /// Returns the path to the settings file.
    ///
    /// Creates the config directory if it doesn't exist.
    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("", "antigravity", "ai-shot").map(|dirs| {
            let config_dir = dirs.config_dir();
            if !config_dir.exists() {
                let _ = fs::create_dir_all(config_dir);
            }
            config_dir.join("settings.json")
        })
    }

    /// Loads settings from disk, falling back to defaults if not found.
    ///
    /// # Arguments
    /// * `default_model` - The model to use if no settings file exists.
    pub fn load(default_model: &str) -> Self {
        Self::config_path()
            .and_then(|path| fs::read_to_string(&path).ok())
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_else(|| Self::with_defaults(default_model))
    }

    /// Creates default settings with the specified model.
    pub fn with_defaults(model: &str) -> Self {
        Self {
            model: model.to_string(),
            system_prompt: String::new(),
            thinking_enabled: false,
            google_search: false,
            api_key: String::new(),
        }
    }

    /// Persists settings to disk.
    ///
    /// # Errors
    /// Returns an error if serialization or file writing fails.
    pub fn save(&self) -> Result<()> {
        if let Some(path) = Self::config_path() {
            let json = serde_json::to_string_pretty(self)?;
            fs::write(path, json)?;
        }
        Ok(())
    }

    /// Returns whether the API key is set (either from settings or will use env).
    pub fn has_api_key(&self) -> bool {
        !self.api_key.is_empty()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::with_defaults("gemini-flash-latest")
    }
}
