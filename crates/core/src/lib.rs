//! AI-Shot Core Library
//!
//! This library provides the core functionality for the AI-Shot screenshot tool,
//! including screen capture, image processing, and Gemini AI integration.
//!
//! # Overview
//!
//! AI-Shot allows users to capture screenshots, select regions of interest,
//! and query Google's Gemini AI about the visual content. The library handles:
//!
//! - **Screen Capture**: Multi-monitor support via the [`capture`] module
//! - **Image Processing**: Region cropping and base64 encoding via [`image_processing`]
//! - **AI Integration**: Gemini API streaming responses via [`gemini`]
//! - **User Interface**: Interactive selection overlay via [`ui`]
//!
//! # Quick Start
//!
//! The simplest way to use the library is through the [`AiShot`] facade:
//!
//! ```ignore
//! use ai_shot_core::AiShot;
//!
//! // Initialize with environment configuration
//! let app = AiShot::new()?;
//!
//! // List available monitors
//! for monitor in app.list_monitors() {
//!     println!("{}", monitor);
//! }
//!
//! // Launch interactive mode on primary monitor
//! app.run_interactive(0)?;
//! ```
//!
//! # Module Structure
//!
//! - [`capture`]: Screen capture functionality
//! - [`config`]: Configuration loading and management
//! - [`error`]: Error types and result aliases
//! - [`gemini`]: Gemini AI client with streaming support
//! - [`image_processing`]: Image manipulation utilities
//! - [`ui`]: User interface components

pub mod capture;
pub mod config;
pub mod error;
pub mod gemini;
pub mod image_processing;
pub mod ui;

// Re-export primary types for convenience
pub use capture::ScreenCapturer;
pub use config::Config;
pub use error::{AppError, Result};
pub use gemini::GeminiClient;

use image::DynamicImage;

/// Main entry point for the AI-Shot application.
///
/// This struct provides a facade over the various subsystems,
/// handling initialization and orchestration. It's the recommended
/// way to use the library for most use cases.
///
/// # Example
///
/// ```ignore
/// use ai_shot_core::AiShot;
///
/// let app = AiShot::new()?;
/// app.run_interactive(0)?;
/// ```
pub struct AiShot {
    config: Config,
    capturer: ScreenCapturer,
}

impl AiShot {
    /// Creates a new AiShot instance with default configuration.
    ///
    /// Loads configuration from environment variables (including `.env` files)
    /// and initializes the screen capturer.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Screen capture initialization fails (e.g., no display available)
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let capturer = ScreenCapturer::new()?;
        Ok(Self { config, capturer })
    }

    /// Creates an instance with custom configuration.
    ///
    /// Use this when you need to override environment-based configuration,
    /// such as specifying a different model or API key.
    ///
    /// # Arguments
    /// * `config` - Pre-built configuration
    ///
    /// # Errors
    ///
    /// Returns an error if screen capture initialization fails.
    pub fn with_config(config: Config) -> Result<Self> {
        let capturer = ScreenCapturer::new()?;
        Ok(Self { config, capturer })
    }

    /// Lists available monitors with their dimensions.
    ///
    /// Returns a vector of human-readable monitor descriptions,
    /// useful for displaying to users or for debugging.
    pub fn list_monitors(&self) -> Vec<String> {
        self.capturer.list_screen()
    }

    /// Returns the number of available monitors.
    pub fn monitor_count(&self) -> usize {
        self.capturer.screen_count()
    }

    /// Captures a specific monitor and launches the interactive UI.
    ///
    /// This is the main entry point for the visual selection workflow.
    /// It captures the specified monitor, displays a fullscreen overlay,
    /// and allows the user to select a region and query Gemini AI.
    ///
    /// # Arguments
    /// * `monitor_index` - Zero-based index of the monitor to capture
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The monitor index is out of bounds
    /// - Screen capture fails
    /// - UI initialization fails
    pub fn run_interactive(&self, monitor_index: usize) -> Result<()> {
        let screenshot = self.capturer.capture_screen_by_index(monitor_index)?;
        ui::run_selection_ui(screenshot, self.config.clone())?;
        Ok(())
    }

    /// Launches the interactive UI with a pre-captured image.
    ///
    /// This is useful when the image has already been captured (e.g., by a daemon)
    /// or loaded from disk.
    pub fn run_interactive_with_image(&self, image: DynamicImage) -> Result<()> {
        ui::run_selection_ui(image, self.config.clone())?;
        Ok(())
    }

    /// Captures a screenshot from a specific monitor without UI.
    ///
    /// Useful for headless operation or when you want to process
    /// the image programmatically.
    ///
    /// # Arguments
    /// * `monitor_index` - Zero-based index of the monitor to capture
    pub fn capture(&self, monitor_index: usize) -> Result<DynamicImage> {
        self.capturer.capture_screen_by_index(monitor_index)
    }

    /// Returns a reference to the current configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a mutable reference to the configuration.
    ///
    /// Allows modifying settings like the model name after initialization.
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }
}

/// Initializes the library by loading environment variables.
///
/// Call this once at application startup before using any other functions.
/// This loads `.env` files if present and sets up the environment.
///
/// # Example
///
/// ```ignore
/// ai_shot_core::init();
/// let config = ai_shot_core::Config::load()?;
/// ```
pub fn init() {
    let _ = dotenvy::dotenv();
}