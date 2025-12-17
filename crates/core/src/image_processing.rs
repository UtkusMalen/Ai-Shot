//! Image processing and encoding utilities.
//!
//! This module handles cropping screen captures based on UI selections
//! and encoding them for transmission to the Gemini API.
//!
//! # Coordinate Mapping
//!
//! The UI displays images at logical pixel sizes (e.g., 1920x1080) while
//! the actual captured image may be at a different resolution (e.g., 3840x2160).
//! This module handles the coordinate transformation between UI space and
//! image space.

use crate::error::{AppError, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use eframe::egui;
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

/// Image processing utilities for the selection workflow.
///
/// This struct provides static methods for processing captured images
/// before sending them to the Gemini API.
pub struct ImageProcessor;

impl ImageProcessor {
    /// Crops an image based on UI selection coordinates and encodes it to Base64.
    ///
    /// This function handles the coordinate transformation from UI logical pixels
    /// to actual image pixels, accounting for HiDPI displays where the image
    /// resolution may be higher than the UI resolution.
    ///
    /// # Arguments
    ///
    /// * `original` - The full captured screenshot
    /// * `selection` - The selected region in UI coordinates
    /// * `ui_size` - The size of the UI display area
    ///
    /// # Returns
    ///
    /// A Base64-encoded JPEG string ready for API transmission.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::EmptySelection`] if the selection has zero area.
    /// Returns [`AppError::ImageProcessing`] if JPEG encoding fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let base64 = ImageProcessor::process_selection(
    ///     &screenshot,
    ///     selection_rect,
    ///     screen_size,
    /// )?;
    /// ```
    pub fn process_selection(
        original: &DynamicImage,
        selection: egui::Rect,
        ui_size: egui::Vec2,
    ) -> Result<String> {
        // Calculate scaling factors between UI and image coordinates
        let scale_x = original.width() as f32 / ui_size.x;
        let scale_y = original.height() as f32 / ui_size.y;

        // Transform UI coordinates to image coordinates
        let x = (selection.min.x * scale_x).max(0.0) as u32;
        let y = (selection.min.y * scale_y).max(0.0) as u32;

        // Calculate dimensions with scaling
        let mut width = (selection.width() * scale_x) as u32;
        let mut height = (selection.height() * scale_y) as u32;

        // Clamp to image bounds to prevent out-of-bounds errors
        if x + width > original.width() {
            width = original.width().saturating_sub(x);
        }
        if y + height > original.height() {
            height = original.height().saturating_sub(y);
        }

        // Validate selection has non-zero area
        if width == 0 || height == 0 {
            return Err(AppError::EmptySelection);
        }

        // Crop the image (immutable operation, returns new image)
        let cropped = original.crop_imm(x, y, width, height);

        // Encode as JPEG
        let base64_string = Self::encode_to_base64_jpeg(&cropped)?;

        Ok(base64_string)
    }

    /// Encodes a DynamicImage to a Base64 JPEG string.
    ///
    /// Uses a reasonable JPEG quality setting for a balance between
    /// file size and image quality.
    fn encode_to_base64_jpeg(image: &DynamicImage) -> Result<String> {
        let mut buffer: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        image
            .write_to(&mut cursor, ImageFormat::Jpeg)
            .map_err(|e| AppError::image(format!("Failed to encode image: {}", e)))?;

        Ok(BASE64.encode(buffer))
    }

    /// Calculates the aspect ratio of an image.
    ///
    /// Returns width divided by height. Useful for maintaining
    /// aspect ratio during resizing operations.
    #[allow(dead_code)]
    pub fn aspect_ratio(image: &DynamicImage) -> f32 {
        image.width() as f32 / image.height() as f32
    }
}