//! Main snipping tool application.
//!
//! This module contains the `SnippingTool` struct which implements the
//! `eframe::App` trait for the fullscreen selection overlay.

use super::rendering::{calculate_popup_position, draw_selection_border, draw_selection_overlay};
use super::selection::{process_drag_event, SelectionEvent};
use super::settings::{Settings, AVAILABLE_MODELS};
use super::state::{SelectionResult, StreamEvent, UiState};
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::gemini::{GeminiClient, GeminiStreamEvent};
use crate::image_processing::ImageProcessor;
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use image::DynamicImage;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

/// The main snipping tool application.
///
/// Displays a fullscreen overlay with the captured screenshot, allowing
/// users to select a region and interact with Gemini AI.
pub struct SnippingTool {
    // Image state
    image_texture: Option<egui::TextureHandle>,
    /// Pre-converted image data for fast texture upload
    color_image: Option<egui::ColorImage>,
    screenshot: DynamicImage,

    // Selection state
    selection_start: Option<egui::Pos2>,
    current_pos: Option<egui::Pos2>,
    is_selection_finalized: bool,
    pub result: Arc<Mutex<SelectionResult>>,

    // Chat state
    chat_input: String,

    // API state
    #[allow(dead_code)]
    config: Config,
    state: UiState,
    rx: Receiver<StreamEvent>,
    tx: Sender<StreamEvent>,

    // Markdown rendering
    markdown_cache: CommonMarkCache,

    // Settings
    settings: Settings,
    show_settings: bool,
}

impl SnippingTool {
    /// Creates a new snipping tool instance.
    ///
    /// # Arguments
    /// * `screenshot` - The captured screen image
    /// * `result` - Shared result container for returning selection to caller
    /// * `config` - Application configuration
    pub fn new(screenshot: DynamicImage, result: Arc<Mutex<SelectionResult>>, config: Config) -> Self {
        let (tx, rx) = channel();

        // Load settings, using config's API key as fallback
        let mut initial_settings = Settings::load(&config.model_name);
        if initial_settings.api_key.is_empty() {
            initial_settings.api_key = config.gemini_api_key.clone();
        }

        // Pre-convert screenshot to ColorImage for fast texture upload
        // This is the expensive operation - do it before the UI loop starts
        let image_buffer = screenshot.to_rgba8();
        let size = [screenshot.width() as usize, screenshot.height() as usize];
        let pixels = image_buffer.as_flat_samples();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

        Self {
            image_texture: None,
            color_image: Some(color_image),
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
            settings: initial_settings,
            show_settings: false,
        }
    }

    /// Submits a request to the Gemini API for image analysis.
    ///
    /// Spawns a background thread to handle the async API call and streams
    /// results back through the channel.
    fn submit_request(&mut self, selection: egui::Rect, ui_size: egui::Vec2, prompt: String) {
        // Save settings before making request
        if let Err(e) = self.settings.save() {
            eprintln!("Warning: Failed to save settings: {}", e);
        }

        self.state = UiState::Response {
            text: String::new(),
            thoughts: String::new(),
        };

        let tx = self.tx.clone();
        let screenshot = self.screenshot.clone();
        let settings = self.settings.clone();

        // Spawn background thread for async work
        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();

            match runtime {
                Ok(rt) => {
                    rt.block_on(async {
                        // Process image to base64
                        let base64_img =
                            match ImageProcessor::process_selection(&screenshot, selection, ui_size)
                            {
                                Ok(img) => img,
                                Err(e) => {
                                    let _ = tx.send(StreamEvent::Error(format!(
                                        "Image processing failed: {}",
                                        e
                                    )));
                                    return;
                                }
                            };

                        // Create Gemini client with current settings
                        let task_config = Config::builder()
                            .with_api_key(&settings.api_key)
                            .with_model(&settings.model)
                            .build();

                        let task_config = match task_config {
                            Ok(c) => c,
                            Err(e) => {
                                let _ = tx.send(StreamEvent::Error(format!(
                                    "Configuration error: {}",
                                    e
                                )));
                                return;
                            }
                        };

                        let client = match GeminiClient::new(&task_config) {
                            Ok(c) => c,
                            Err(e) => {
                                let _ = tx.send(StreamEvent::Error(format!(
                                    "Client initialization failed: {}",
                                    e
                                )));
                                return;
                            }
                        };

                        // Stream response from Gemini
                        match client
                            .analyze_image_stream(
                                base64_img,
                                prompt,
                                settings.system_prompt,
                                settings.thinking_enabled,
                                settings.google_search,
                            )
                            .await
                        {
                            Ok(mut stream) => {
                                use futures::StreamExt;

                                while let Some(result) = stream.next().await {
                                    match result {
                                        Ok(events) => {
                                            for event in events {
                                                match event {
                                                    GeminiStreamEvent::Text(text) => {
                                                        let _ = tx.send(StreamEvent::Chunk(text));
                                                    }
                                                    GeminiStreamEvent::Thought(thought) => {
                                                        let _ =
                                                            tx.send(StreamEvent::Thought(thought));
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let _ = tx.send(StreamEvent::Error(format!(
                                                "Stream error: {}",
                                                e
                                            )));
                                        }
                                    }
                                }
                                let _ = tx.send(StreamEvent::Done);
                            }
                            Err(e) => {
                                let _ =
                                    tx.send(StreamEvent::Error(format!("Gemini API error: {}", e)));
                            }
                        }
                    });
                }
                Err(e) => {
                    let _ = tx.send(StreamEvent::Error(format!(
                        "Failed to create async runtime: {}",
                        e
                    )));
                }
            }
        });
    }

    /// Processes stream events from the background thread.
    fn process_stream_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                StreamEvent::Chunk(text) => {
                    if let UiState::Response {
                        text: current_text,
                        ..
                    } = &mut self.state
                    {
                        current_text.push_str(&text);
                    } else {
                        self.state = UiState::Response {
                            text,
                            thoughts: String::new(),
                        };
                    }
                    ctx.request_repaint();
                }
                StreamEvent::Thought(thought) => {
                    if let UiState::Response { thoughts, .. } = &mut self.state {
                        thoughts.push_str(&thought);
                    } else {
                        self.state = UiState::Response {
                            text: String::new(),
                            thoughts: thought,
                        };
                    }
                    ctx.request_repaint();
                }
                StreamEvent::Error(err) => {
                    self.state = UiState::Error(err);
                }
                StreamEvent::Done => {
                    // Stream completed - could trigger analytics or logging here
                }
            }
        }
    }

    /// Renders the idle state UI (prompt input).
    fn render_idle_ui(&mut self, ui: &mut egui::Ui, selection_rect: egui::Rect) {
        ui.horizontal(|ui| {
            ui.label("Ask Gemini:");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.chat_input)
                    .desired_width(200.0)
                    .hint_text("e.g., Explain this code")
                    .lock_focus(true),
            );

            if !self.show_settings {
                response.request_focus();
            }

            let enter_pressed = response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("➤").clicked() || enter_pressed {
                let prompt = if self.chat_input.trim().is_empty() {
                    "Explain this image in detail.".to_string()
                } else {
                    self.chat_input.clone()
                };

                self.submit_request(selection_rect, ui.ctx().viewport_rect().size(), prompt);
            }

            if ui.button("⚙").clicked() {
                self.show_settings = !self.show_settings;
            }
        });

        if self.show_settings {
            self.render_settings_ui(ui);
        }
    }

    /// Renders the settings panel.
    fn render_settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label("Settings");

        // Model selector
        egui::ComboBox::from_label("Model")
            .selected_text(&self.settings.model)
            .show_ui(ui, |ui| {
                for model in AVAILABLE_MODELS {
                    ui.selectable_value(&mut self.settings.model, model.to_string(), *model);
                }
            });

        // Feature toggles
        ui.checkbox(&mut self.settings.thinking_enabled, "Enable Thinking");
        ui.checkbox(&mut self.settings.google_search, "Use Google Search");

        // API Key
        ui.label("API Key:");
        ui.add(
            egui::TextEdit::singleline(&mut self.settings.api_key)
                .password(true)
                .hint_text("Paste Gemini API Key"),
        );

        // System prompt
        ui.label("System Instructions:");
        ui.add(
            egui::TextEdit::multiline(&mut self.settings.system_prompt)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
    }

    /// Renders the response state UI.
    fn render_response_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, text: &str, thoughts: &str) {
        ui.horizontal(|ui| {
            ui.heading("Gemini says:");
            if text.is_empty() && thoughts.is_empty() {
                ui.spinner();
            }
        });

        // Display thoughts if available
        if !thoughts.is_empty() {
            egui::CollapsingHeader::new("Thinking Process")
                .default_open(true)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .id_salt("thoughts_scroll")
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(thoughts)
                                    .monospace()
                                    .small()
                                    .color(egui::Color32::LIGHT_GRAY),
                            );
                        });
                });
            ui.add_space(8.0);
        }

        // Display response with markdown
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                CommonMarkViewer::new().show(ui, &mut self.markdown_cache, text);
            });

        ui.separator();

        // Action buttons
        let mut should_go_back = false;
        ui.horizontal(|ui| {
            if ui.button("Copy").clicked() {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(text);
                }
            }
            if ui.button("Close").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if ui.button("Back").clicked() {
                should_go_back = true;
            }
        });

        if should_go_back {
            self.state = UiState::Idle;
        }
    }

    /// Renders the error state UI.
    fn render_error_ui(&mut self, ui: &mut egui::Ui, error: &str) {
        ui.label(egui::RichText::new(format!("Error: {}", error)).color(egui::Color32::RED));
        if ui.button("Back").clicked() {
            self.state = UiState::Idle;
        }
    }
}

impl eframe::App for SnippingTool {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Enforce dark mode
        ctx.set_visuals(egui::Visuals::dark());

        // Process any pending stream events
        self.process_stream_events(ctx);

        // Upload texture on first frame using pre-converted data
        if self.image_texture.is_none() {
            if let Some(color_image) = self.color_image.take() {
                self.image_texture = Some(ctx.load_texture(
                    "screenshot",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        // Fullscreen panel with no margins
        let panel_frame = egui::Frame::default()
            .inner_margin(egui::Margin::same(0.0 as i8))
            .outer_margin(egui::Margin::same(0.0 as i8));

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                let rect = ui.max_rect();

                // Draw screenshot as background
                if let Some(texture) = &self.image_texture {
                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }

                // Handle selection input (unless loading)
                if !matches!(self.state, UiState::Loading) {
                    let response = ui.interact(rect, ui.id(), egui::Sense::drag());

                    let event = process_drag_event(
                        &response,
                        &mut self.selection_start,
                        &mut self.current_pos,
                        self.is_selection_finalized,
                    );

                    match event {
                        SelectionEvent::Started => {
                            self.is_selection_finalized = false;
                            self.chat_input.clear();
                            if matches!(self.state, UiState::Response { .. } | UiState::Error(_)) {
                                self.state = UiState::Idle;
                            }
                        }
                        SelectionEvent::Completed => {
                            self.is_selection_finalized = true;
                        }
                        _ => {}
                    }
                }

                // Handle escape to close
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                // Get current interaction position for drawing
                let current_interaction_pos = if self.is_selection_finalized {
                    self.current_pos
                } else {
                    ctx.pointer_interact_pos().or(self.current_pos)
                };

                // Draw selection overlay and UI
                if let (Some(start), Some(current)) = (self.selection_start, current_interaction_pos)
                {
                    let selection_rect = egui::Rect::from_two_pos(start, current);
                    let screen_rect = ui.max_rect();

                    // Draw dark overlay with cutout
                    draw_selection_overlay(ui.painter(), screen_rect, selection_rect, 150);

                    // Draw selection border
                    draw_selection_border(ui.painter(), selection_rect, 2.0, egui::Color32::WHITE);

                    // Show interaction window when selection is finalized
                if self.is_selection_finalized {
                    // responsive width: 30% of screen width, clamped between 400 and 800
                    let window_width = (screen_rect.width() * 0.3).clamp(400.0, 800.0);
                    let (window_x, window_y, pivot) = calculate_popup_position(
                        selection_rect,
                        screen_rect,
                        window_width,
                        10.0,
                        400.0,
                    );
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

                                        // Clone state data for rendering to avoid borrow issues
                                        let state_clone = self.state.clone();
                                        match state_clone {
                                            UiState::Idle => {
                                                self.render_idle_ui(ui, selection_rect);
                                            }
                                            UiState::Loading => {
                                                ui.horizontal(|ui| {
                                                    ui.spinner();
                                                    ui.label("Analyzing...");
                                                });
                                            }
                                            UiState::Response { text, thoughts } => {
                                                self.render_response_ui(ui, ctx, &text, &thoughts);
                                            }
                                            UiState::Error(err) => {
                                                self.render_error_ui(ui, &err);
                                            }
                                        }
                                    });
                            });
                    }
                }
            });
    }
}

/// Launches the selection UI and returns when the user closes the window.
///
/// # Arguments
/// * `screenshot` - The captured screen image
/// * `config` - Application configuration
///
/// # Returns
/// The selected rectangle and screen size, or `None` if cancelled.
pub fn run(
    screenshot: DynamicImage,
    config: Config,
) -> Result<Option<(egui::Rect, egui::Vec2, Option<String>)>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_fullscreen(true)
            .with_decorations(false)
            .with_always_on_top(),
        ..Default::default()
    };

    let result = Arc::new(Mutex::new(SelectionResult::default()));
    let app_result = result.clone();

    eframe::run_native(
        "Screen Gemini Selection",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(SnippingTool::new(screenshot, app_result, config)) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|e| AppError::ui(format!("Failed to run UI: {}", e)))?;

    // Extract result from shared state
    let lock = result
        .lock()
        .map_err(|_| AppError::ui("Failed to acquire result lock"))?;

    match (lock.selected_area, lock.screen_size) {
        (Some(area), Some(size)) => Ok(Some((area, size, lock.user_prompt.clone()))),
        _ => Ok(None),
    }
}
