use crate::error::{AppError, Result};
use eframe::egui;
use image::DynamicImage;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SelectionResult {
    pub selected_area: Option<egui::Rect>,
    pub screen_size: Option<egui::Vec2>,
}

pub struct SnippingTool {
    image_texture: Option<egui::TextureHandle>,
    screenshot: DynamicImage,
    selection_start: Option<egui::Pos2>,
    current_pos: Option<egui::Pos2>,
    pub result: Arc<Mutex<SelectionResult>>,
}

impl SnippingTool {
    pub fn new(screenshot: DynamicImage, result: Arc<Mutex<SelectionResult>>) -> Self {
        Self {
            image_texture: None,
            screenshot,
            selection_start: None,
            current_pos: None,
            result
        }
    }
}

impl eframe::App for SnippingTool {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Load texture if not loaded
        if self.image_texture.is_none() {
            let image_buffer = self.screenshot.to_rgba8();
            let size = [self.screenshot.width() as usize, self.screenshot.height() as usize];
            let pixels = image_buffer.as_flat_samples();

            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                size,
                pixels.as_slice()
            );

            self.image_texture = Some(ctx.load_texture(
                "screenshot",
                color_image,
                egui::TextureOptions::LINEAR
            ));
        }

        // Make the window fullscreen and borderless
        let panel_frame = egui::Frame::default()
            .inner_margin(egui::Margin::same(0.0 as i8))
            .outer_margin(egui::Margin::same(0.0 as i8));

        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            let rect = ui.max_rect();

            // Draw the screenshot as background
            if let Some(texture) = &self.image_texture {
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE
                );
            }

            // Handle inputs
            let response = ui.interact(rect, ui.id(), egui::Sense::drag());

            if response.drag_started() {
                self.selection_start = response.interact_pointer_pos();
            }

            if response.dragged() {
                self.current_pos = response.interact_pointer_pos();
            }

            if response.drag_stopped() {
                // Calculate final selection
                if let (Some(start), Some(end)) = (self.selection_start, self.current_pos) {
                    let selection = egui::Rect::from_two_pos(start, end);
                    // Save result
                    if let Ok(mut res) = self.result.lock() {
                        res.selected_area = Some(selection);
                        res.screen_size = Some(rect.size()); // Save the UI dimensions
                    }
                    // Close window
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }

            // Handle escape to cancel
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            // Draw selection overlay
            if let (Some(start), Some(current)) = (self.selection_start, ctx.pointer_interact_pos()) {
                let selection_rect = egui::Rect::from_two_pos(start, current);

                // Construct a "cutout" effect by drawing 4 dart rectangles around the selection
                let screen_rect = ui.max_rect();
                let color = egui::Color32::from_black_alpha(150);

                // Top
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(screen_rect.min, egui::pos2(screen_rect.max.x, selection_rect.min.y)),
                    0.0,
                    color,
                );
                // Bottom
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(egui::pos2(screen_rect.min.x, selection_rect.max.y), screen_rect.max),
                    0.0,
                    color,
                );
                // Left
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(screen_rect.min.x, selection_rect.min.y),
                        egui::pos2(selection_rect.min.x, selection_rect.max.y)
                    ),
                    0.0,
                    color,
                );
                // Right
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(selection_rect.max.x, selection_rect.min.y),
                        egui::pos2(screen_rect.max.x, selection_rect.max.y)
                    ),
                    0.0,
                    color,
                );

                // Draw border around selection
                ui.painter().rect_stroke(
                    selection_rect,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                    egui::StrokeKind::Middle,
                );
            }
        });
    }
}

/// Helper function to launch the UI and return the selected rectangle
pub fn run_selection_ui(screenshot: DynamicImage) -> Result<Option<(egui::Rect, egui::Vec2)>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_fullscreen(true)
            .with_decorations(false)
            .with_always_on_top(),
        ..Default::default()
    };

    let result = Arc::new(Mutex::new(SelectionResult { 
        selected_area: None,
        screen_size: None,
    }));
    let app_result = result.clone();
    let app_screenshot = screenshot.clone();

    eframe::run_native(
        "Screen Gemini Selection",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(SnippingTool::new(app_screenshot, app_result)) as Box<dyn eframe::App>)
        }),
    ).map_err(|e| AppError::Ui(format!("Failed to run UI: {}", e)))?;

    let lock = result.lock().unwrap();
    match (lock.selected_area, lock.screen_size) {
        (Some(area), Some(size)) => Ok(Some((area, size))),
        _ => Ok(None),
    }
}