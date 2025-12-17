//! User interface components for ai-shot.
//!
//! This module provides the snipping tool overlay for screen region selection
//! and AI-powered image analysis using Google's Gemini API.
//!
//! # Architecture
//!
//! The UI is split into focused submodules:
//! - [`state`]: State machine types and event definitions
//! - [`settings`]: User preferences and persistence
//! - [`rendering`]: Drawing utilities for overlays and borders
//! - [`selection`]: User interaction handling
//! - [`snipping_tool`]: Main application logic
//!
//! # Usage
//!
//! ```ignore
//! use ai_shot_core::ui;
//! use ai_shot_core::Config;
//!
//! let config = Config::load()?;
//! let screenshot = capture_screen()?;
//!
//! // Launch the interactive selection UI
//! if let Some((rect, size, prompt)) = ui::run_selection_ui(screenshot, config)? {
//!     // User made a selection
//! }
//! ```

mod rendering;
mod selection;
mod settings;
mod snipping_tool;
mod state;

// Public API exports
pub use settings::{Settings, AVAILABLE_MODELS};
pub use snipping_tool::SnippingTool;
pub use state::{SelectionResult, UiState};

use crate::config::Config;
use crate::error::Result;
use image::DynamicImage;

/// Launches the selection UI and returns the user's selection.
///
/// This function displays a fullscreen overlay with the captured screenshot,
/// allowing users to select a region and optionally query Gemini AI about it.
///
/// # Arguments
/// * `screenshot` - The captured screen as a [`DynamicImage`]
/// * `config` - Application configuration with API keys and settings
///
/// # Returns
/// - `Ok(Some((rect, size, prompt)))` - User made a valid selection
/// - `Ok(None)` - User cancelled (pressed Escape)
/// - `Err(e)` - An error occurred launching or running the UI
///
/// # Example
/// ```ignore
/// let result = ui::run_selection_ui(screenshot, config)?;
/// if let Some((selection, screen_size, prompt)) = result {
///     println!("Selected: {:?}", selection);
/// }
/// ```
pub fn run_selection_ui(
    screenshot: DynamicImage,
    config: Config,
) -> Result<Option<(eframe::egui::Rect, eframe::egui::Vec2, Option<String>)>> {
    snipping_tool::run(screenshot, config)
}
