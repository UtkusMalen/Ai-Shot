use crate::error::{AppError, Result};
use eframe::egui;
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub struct ImageProcessor;

impl ImageProcessor {
    /// Crops the image based on UI coordinates and encodes it to Base64
    pub fn process_selection(
        original: &DynamicImage,
        selection: egui::Rect,
        ui_size: egui::Vec2
    ) -> Result<String> {
        // Calculate scaling factor
        // UI might be in logical points (e.g., 1920x1080) while the image is raw pixels (e.g. 3840x2160)
        let scale_x = original.width() as f32 / ui_size.x;
        let scale_y = original.height() as f32 / ui_size.y;

        // Map coordinates
        let x = (selection.min.x * scale_x).max(0.0) as u32;
        let y = (selection.min.y * scale_y).max(0.0) as u32;

        let mut width = (selection.width() * scale_x) as u32;
        let mut height = (selection.height() * scale_y) as u32;

        // Bounds checking
        // Ensure we don't try to crop outside the image
        if x + width > original.width() {
            width = original.width().saturating_sub(x);
        }
        if y + height > original.height() {
            height = original.height().saturating_sub(y);
        }

        if width == 0 || height == 0 {
            return Err(AppError::ImageProcessing("Select area is empty".to_string()));
        }

        // Crop
        // crop_imm is immutable and returns a new image
        let cropped = original.crop_imm(x, y, width, height);

        // Encode the JPEG
        let mut image_data: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(&mut image_data);

        cropped.write_to(&mut cursor, ImageFormat::Jpeg)
            .map_err(|e| AppError::ImageProcessing(format!("Failed to encode image: {}", e)))?;

        // Convert to Base64
        let base64_string = BASE64.encode(image_data);

        Ok(base64_string)
    }
}