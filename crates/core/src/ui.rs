use crate::error::{AppError, Result};
use crate::config::Config;
use crate::gemini::GeminiClient;
use crate::image_processing::ImageProcessor;
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use image::DynamicImage;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[derive(Clone)]
pub struct SelectionResult {
    pub selected_area: Option<egui::Rect>,
    pub screen_size: Option<egui::Vec2>,
    pub user_prompt: Option<String>,
}

#[derive(PartialEq)]
enum UiState {
    Idle,
    Loading,
    Response(String),
    Error(String),
}

pub struct SnippingTool {
    image_texture: Option<egui::TextureHandle>,
    screenshot: DynamicImage,
    selection_start: Option<egui::Pos2>,
    current_pos: Option<egui::Pos2>,
    pub result: Arc<Mutex<SelectionResult>>,
    
    // Chat window state
    chat_input: String,
    is_selection_finalized: bool,
    
    // API State
    config: Config,
    state: UiState,
    rx: Receiver<StreamEvent>,
    tx: Sender<StreamEvent>,
    
    // Markdown
    markdown_cache: CommonMarkCache,
}

enum StreamEvent {
    Chunk(String),
    Error(String),
    Done,
}

impl SnippingTool {
    pub fn new(screenshot: DynamicImage, result: Arc<Mutex<SelectionResult>>, config: Config) -> Self {
        let (tx, rx) = channel();
        Self {
            image_texture: None,
            screenshot,
            selection_start: None,
            current_pos: None,
            result,
            chat_input: String::new(),
            is_selection_finalized: false,
            config,
            state: UiState::Idle,
            rx,
            tx,
            markdown_cache: CommonMarkCache::default(),
        }
    }

    fn submit_request(&mut self, selection: egui::Rect, ui_size: egui::Vec2, prompt: String) {
        self.state = UiState::Loading;
        let tx = self.tx.clone();
        let screenshot = self.screenshot.clone();
        let config = self.config.clone();
        
        // Spawn a thread to handle the heavy lifting
        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();

            match runtime {
                Ok(rt) => {
                    rt.block_on(async {
                        // 1. Process Image
                        let base64_img = match ImageProcessor::process_selection(&screenshot, selection, ui_size) {
                            Ok(img) => img,
                            Err(e) => {
                                let _ = tx.send(StreamEvent::Error(format!("Image processing failed: {}", e)));
                                return;
                            }
                        };
                        
                        // 2. Call API
                        let client = match GeminiClient::new(&config) {
                             Ok(c) => c,
                             Err(e) => {
                                 let _ = tx.send(StreamEvent::Error(format!("Client init failed: {}", e)));
                                 return;
                             }
                        };
                            
                        match client.analyze_image_stream(base64_img, prompt).await {
                            Ok(mut stream) => {
                                use futures::StreamExt;
                                while let Some(result) = stream.next().await {
                                    match result {
                                        Ok(text) => {
                                            let _ = tx.send(StreamEvent::Chunk(text));
                                        },
                                        Err(e) => {
                                            let _ = tx.send(StreamEvent::Error(format!("Stream error: {}", e)));
                                        }
                                    }
                                }
                                let _ = tx.send(StreamEvent::Done);
                            },
                            Err(e) => {
                                let _ = tx.send(StreamEvent::Error(format!("Gemini API Error: {}", e)));
                            }
                        }
                    });
                }
                Err(e) => {
                    let _ = tx.send(StreamEvent::Error(format!("Failed to create runtime: {}", e)));
                }
            }
        });
    }
}

impl eframe::App for SnippingTool {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for async results
        while let Ok(event) = self.rx.try_recv() {
            match event {
                StreamEvent::Chunk(text) => {
                    match &mut self.state {
                        UiState::Response(current) => {
                            current.push_str(&text);
                        },
                        _ => {
                            self.state = UiState::Response(text);
                        }
                    }
                    // Force repaint on new update
                    ctx.request_repaint();
                },
                StreamEvent::Error(err) => {
                     // If we are already streaming, we might want to append the error or show it differently
                     // For now, just switch to Error state
                     self.state = UiState::Error(err);
                },
                StreamEvent::Done => {
                    // Start a new line or finalize if needed
                }
            }
        }

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

            // Handle inputs - allow re-selection unless loading
            if !matches!(self.state, UiState::Loading) {
                let response = ui.interact(rect, ui.id(), egui::Sense::drag());

                if response.drag_started() {
                    self.is_selection_finalized = false;
                    self.selection_start = response.interact_pointer_pos();
                    self.current_pos = response.interact_pointer_pos();
                    self.chat_input.clear();
                    // If we were viewing a response/error, reset to Idle
                    if matches!(self.state, UiState::Response(_) | UiState::Error(_)) {
                         self.state = UiState::Idle;
                    }
                }

                if response.dragged() {
                    self.current_pos = response.interact_pointer_pos();
                }

                if response.drag_stopped() {
                     if let (Some(start), Some(end)) = (self.selection_start, self.current_pos) {
                         if start.distance(end) > 10.0 {
                             self.is_selection_finalized = true;
                         } else {
                             self.selection_start = None;
                             self.current_pos = None;
                         }
                    }
                }
            }


            // Handle escape to cancel
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            // Draw selection overlay
            let current_interaction_pos = if self.is_selection_finalized {
                self.current_pos
            } else {
                 ctx.pointer_interact_pos().or(self.current_pos)
            };

            if let (Some(start), Some(current)) = (self.selection_start, current_interaction_pos) {
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

                // UI Window (Chat or Result)
                if self.is_selection_finalized {
                    let window_width = 400.0;
                    let spacing = 10.0;
                    
                    // Center horizontally relative to selection, but clamp to screen bounds
                    let mut window_x = selection_rect.center().x - (window_width / 2.0);
                    window_x = window_x.clamp(10.0, screen_rect.width() - window_width - 10.0);

                    // Position logic
                    let mut pivot = egui::Align2::LEFT_TOP;
                    let mut window_y = selection_rect.max.y + spacing;
                    
                    let space_below = screen_rect.max.y - window_y;
                    
                    // If less than 400px below, check if we have more space above
                    if space_below < 400.0 && selection_rect.min.y > space_below {
                        pivot = egui::Align2::LEFT_BOTTOM;
                        window_y = selection_rect.min.y - spacing;
                    }

                    egui::Area::new(egui::Id::new("interaction_area"))
                        .fixed_pos(egui::pos2(window_x, window_y))
                        .pivot(pivot)
                        .show(ctx, |ui| {
                             egui::Frame::popup(ui.style())
                                .fill(egui::Color32::from_rgb(30, 30, 30))
                                .stroke(egui::Stroke::new(1.0, egui::Color32::GRAY))
                                .inner_margin(10.0)
                                .show(ui, |ui| {
                                    ui.set_width(window_width);
                                    
                                    let mut next_state = None;
                                    
                                    match &self.state {
                                        UiState::Idle => {
                                             ui.horizontal(|ui| {
                                                ui.label("Ask Gemini:");
                                                let response = ui.add(
                                                    egui::TextEdit::singleline(&mut self.chat_input)
                                                        .desired_width(220.0)
                                                        .hint_text("e.g., Explain this code")
                                                        .lock_focus(true)
                                                );
                                                
                                                response.request_focus();

                                                if ui.button("➤").clicked() || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                                                     // Submit
                                                     let prompt = if self.chat_input.trim().is_empty() {
                                                         "Explain this image in detail.".to_string()
                                                     } else {
                                                         self.chat_input.clone()
                                                     };
                                                     
                                                     self.submit_request(
                                                         selection_rect, 
                                                         ui.ctx().viewport_rect().size(),
                                                         prompt
                                                     );
                                                }
                                            });
                                        },
                                        UiState::Loading => {
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.label("Analyzing...");
                                            });
                                        },
                                        UiState::Response(text) => {
                                            ui.heading("Gemini says:");
                                            egui::ScrollArea::vertical()
                                                .max_height(300.0)
                                                .show(ui, |ui| {
                                                    CommonMarkViewer::new()
                                                        .show(ui, &mut self.markdown_cache, text);
                                                });
                                                
                                            ui.separator();
                                            ui.horizontal(|ui| {
                                                if ui.button("Copy").clicked() {
                                                     if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                                         let _ = clipboard.set_text(text.clone());
                                                     }
                                                }
                                                if ui.button("Close").clicked() {
                                                     ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                                }
                                                if ui.button("Back").clicked() {
                                                    next_state = Some(UiState::Idle);
                                                }
                                            });
                                        },
                                        UiState::Error(err) => {
                                            ui.label(egui::RichText::new(format!("Error: {}", err)).color(egui::Color32::RED));
                                            if ui.button("Back").clicked() {
                                                next_state = Some(UiState::Idle);
                                            }
                                        }
                                    }
                                    
                                    if let Some(ns) = next_state {
                                        self.state = ns;
                                    }
                                });
                        });
                }
            }
        });
    }
}

/// Helper function to launch the UI and return the selected rectangle
pub fn run_selection_ui(screenshot: DynamicImage, config: Config) -> Result<Option<(egui::Rect, egui::Vec2, Option<String>)>> {
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
        user_prompt: None,
    }));
    let app_result = result.clone();
    let app_screenshot = screenshot.clone();

    eframe::run_native(
        "Screen Gemini Selection",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(SnippingTool::new(app_screenshot, app_result, config)) as Box<dyn eframe::App>)
        }),
    ).map_err(|e| AppError::Ui(format!("Failed to run UI: {}", e)))?;

    let lock = result.lock().unwrap();
    match (lock.selected_area, lock.screen_size) {
        (Some(area), Some(size)) => Ok(Some((area, size, lock.user_prompt.clone()))),
        _ => Ok(None),
    }
}