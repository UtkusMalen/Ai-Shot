use crate::error::{AppError, Result};
use image::DynamicImage;
use screenshots::Screen;

/// Screen capturer that works on both X11 and Wayland
pub struct ScreenCapturer {
    screens: Vec<Screen>,
}

impl ScreenCapturer {
    /// Initialize the screen capturer by detecting available screens
    pub fn new() -> Result<Self> {
        let screens = Screen::all()
            .map_err(|e| AppError::ScreenCapture(format!("Failed to enumerate screens: {}", e)))?;

        if screens.is_empty() {
            return Err(AppError::ScreenCapture("No screens detected".to_string()));
        }

        Ok(Self { screens })
    }

    /// List available screens with their dimensions
    pub fn list_screen(&self) -> Vec<String> {
        self.screens.iter().enumerate().map(|(i, s)| {
            format!("Monitor {}: {}x{} (scale: {})",
                i,
                s.display_info.width,
                s.display_info.height,
                s.display_info.scale_factor
            )
        }).collect()
    }

    /// Capture the primary screen (first screen detected)
    pub fn capture_screen(&self) -> Result<DynamicImage> {
        let screen = self.screens.first()
            .ok_or_else(|| AppError::ScreenCapture("No screens available".to_string()))?;

        let image = screen.capture()
            .map_err(|e| AppError::ScreenCapture(format!("Failed to capture screen: {}", e)))?;

        // Convert from screenshots::Image to image::DynamicImage
        let width = image.width();
        let height = image.height();
        let rgba_data = image.into_raw();

        // The screenshots crate returns RGBA data
        let img_buffer = image::ImageBuffer::from_raw(width, height, rgba_data)
            .ok_or_else(|| AppError::ScreenCapture("Failed to create image buffer".to_string()))?;

        Ok(DynamicImage::ImageRgba8(img_buffer))
    }

    /// Capture a specific screen by index
    pub fn capture_screen_by_index(&self, index: usize) -> Result<DynamicImage> {
        let screen = self.screens.get(index)
            .ok_or_else(|| AppError::ScreenCapture(format!("Screen {} not found", index)))?;

        let image = screen.capture()
            .map_err(|e| AppError::ScreenCapture(format!("Failed to capture screen: {}", e)))?;

        let width = image.width();
        let height = image.height();
        let rgba_data = image.into_raw();

        let img_buffer = image::ImageBuffer::from_raw(width, height, rgba_data)
            .ok_or_else(|| AppError::ScreenCapture("Failed to create image buffer".to_string()))?;

        Ok(DynamicImage::ImageRgba8(img_buffer))
    }

    /// Capture a rectangular region from the primary screen
    pub fn capture_region(&self, x: i32, y: i32, width: u32, height: u32) -> Result<DynamicImage> {
        let screen = self.screens.first()
            .ok_or_else(|| AppError::ScreenCapture("No screens available".to_string()))?;

        let image = screen.capture_area(x, y, width, height)
            .map_err(|e| AppError::ScreenCapture(format!("Failed to capture region: {}", e)))?;

        let img_width = image.width();
        let img_height = image.height();
        let rgba_data = image.into_raw();

        let img_buffer = image::ImageBuffer::from_raw(img_width, img_height, rgba_data)
            .ok_or_else(|| AppError::ScreenCapture("Failed to create image buffer".to_string()))?;

        Ok(DynamicImage::ImageRgba8(img_buffer))
    }

    /// Get the number of available screens
    pub fn screen_count(&self) -> usize {
        self.screens.len()
    }

    /// Get screen dimensions for the primary screen
    pub fn primary_screen_dimensions(&self) -> Option<(u32, u32)> {
        self.screens.first().map(|s| (s.display_info.width, s.display_info.height))
    }
}