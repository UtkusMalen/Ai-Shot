//! Screen capture functionality.
//!
//! This module provides cross-platform screen capture capabilities,
//! supporting both X11 and Wayland on Linux, as well as Windows and macOS.
//!
//! # Example
//!
//! ```ignore
//! use ai_shot_core::capture::ScreenCapturer;
//!
//! let capturer = ScreenCapturer::new()?;
//!
//! // List available screens
//! for screen in capturer.list_screen() {
//!     println!("{}", screen);
//! }
//!
//! // Capture the primary screen
//! let screenshot = capturer.capture_screen()?;
//! ```

use crate::error::{AppError, Result};
use image::DynamicImage;
use screenshots::Screen;

/// Screen capturer that provides multi-monitor screenshot capabilities.
///
/// This struct wraps the `screenshots` crate and provides a convenient API
/// for capturing entire screens or specific regions.
///
/// # Thread Safety
///
/// The capturer can be used from multiple threads, but each capture operation
/// must complete before another can begin on the same screen.
pub struct ScreenCapturer {
    screens: Vec<Screen>,
}

impl ScreenCapturer {
    /// Initializes the screen capturer by detecting available screens.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::ScreenCapture`] if:
    /// - Screen enumeration fails (e.g., no display server available)
    /// - No screens are detected
    pub fn new() -> Result<Self> {
        let screens = Screen::all()
            .map_err(|e| AppError::capture(format!("Failed to enumerate screens: {}", e)))?;

        if screens.is_empty() {
            return Err(AppError::capture("No screens detected"));
        }

        Ok(Self { screens })
    }

    /// Lists available screens with their dimensions and metadata.
    ///
    /// Returns a vector of human-readable screen descriptions including
    /// resolution and scale factor.
    pub fn list_screen(&self) -> Vec<String> {
        self.screens
            .iter()
            .enumerate()
            .map(|(i, s)| {
                format!(
                    "Monitor {}: {}x{} (scale: {})",
                    i, s.display_info.width, s.display_info.height, s.display_info.scale_factor
                )
            })
            .collect()
    }

    /// Captures the primary screen (first detected screen).
    ///
    /// # Errors
    ///
    /// Returns [`AppError::ScreenCapture`] if the capture operation fails.
    pub fn capture_screen(&self) -> Result<DynamicImage> {
        self.capture_screen_by_index(0)
    }

    /// Captures a specific screen by its index.
    ///
    /// # Arguments
    /// * `index` - Zero-based index of the screen to capture
    ///
    /// # Errors
    ///
    /// Returns:
    /// - [`AppError::ScreenNotFound`] if the index is out of bounds
    /// - [`AppError::ScreenCapture`] if the capture operation fails
    pub fn capture_screen_by_index(&self, index: usize) -> Result<DynamicImage> {
        let screen = self
            .screens
            .get(index)
            .ok_or(AppError::ScreenNotFound(index))?;

        let captured = screen
            .capture()
            .map_err(|e| AppError::capture(format!("Failed to capture screen: {}", e)))?;

        // Convert screenshots::Image to image::DynamicImage
        let width = captured.width();
        let height = captured.height();
        let rgba_data = captured.into_raw();

        let img_buffer = image::ImageBuffer::from_raw(width, height, rgba_data)
            .ok_or_else(|| AppError::capture("Failed to create image buffer"))?;

        Ok(DynamicImage::ImageRgba8(img_buffer))
    }

    /// Captures a rectangular region from the primary screen.
    ///
    /// # Arguments
    /// * `x` - X coordinate of the top-left corner
    /// * `y` - Y coordinate of the top-left corner
    /// * `width` - Width of the region in pixels
    /// * `height` - Height of the region in pixels
    ///
    /// # Errors
    ///
    /// Returns [`AppError::ScreenCapture`] if the capture operation fails
    /// or the region is invalid.
    pub fn capture_region(&self, x: i32, y: i32, width: u32, height: u32) -> Result<DynamicImage> {
        let screen = self
            .screens
            .first()
            .ok_or_else(|| AppError::capture("No screens available"))?;

        let captured = screen
            .capture_area(x, y, width, height)
            .map_err(|e| AppError::capture(format!("Failed to capture region: {}", e)))?;

        // Convert screenshots::Image to image::DynamicImage
        let img_width = captured.width();
        let img_height = captured.height();
        let rgba_data = captured.into_raw();

        let img_buffer = image::ImageBuffer::from_raw(img_width, img_height, rgba_data)
            .ok_or_else(|| AppError::capture("Failed to create image buffer"))?;

        Ok(DynamicImage::ImageRgba8(img_buffer))
    }

    /// Returns the number of available screens.
    pub fn screen_count(&self) -> usize {
        self.screens.len()
    }

    /// Gets the dimensions of the primary screen.
    ///
    /// Returns `None` if no screens are available.
    pub fn primary_screen_dimensions(&self) -> Option<(u32, u32)> {
        self.screens
            .first()
            .map(|s| (s.display_info.width, s.display_info.height))
    }
}