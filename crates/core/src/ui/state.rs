//! UI state types and event definitions.
//!
//! This module contains the core state machine and event types used by the UI.

use eframe::egui;

/// Result of a screen selection operation.
///
/// This struct captures all the information needed to process a user's selection,
/// including the selected region, screen dimensions, and optional prompt.
#[derive(Clone, Default)]
pub struct SelectionResult {
    /// The selected rectangular area in UI coordinates.
    pub selected_area: Option<egui::Rect>,
    /// The screen size at the time of selection (for coordinate mapping).
    pub screen_size: Option<egui::Vec2>,
    /// Optional user prompt for the AI analysis.
    pub user_prompt: Option<String>,
}

/// Current state of the UI application.
///
/// The UI follows a simple state machine:
/// `Idle` -> `Response` (streaming) -> `Idle` (on back) or closed
///         \-> `Error` (on failure) -> `Idle` (on back)
#[derive(Clone, Debug)]
pub enum UiState {
    /// Waiting for user input (prompt entry).
    Idle,
    /// Loading/processing request (legacy state, kept for compatibility).
    Loading,
    /// Displaying streaming or complete response from Gemini.
    Response {
        /// The accumulated response text.
        text: String,
        /// Thinking process output (if enabled).
        thoughts: String,
    },
    /// An error occurred during processing.
    Error(String),
}

/// Events received from the background streaming task.
///
/// These events are sent through a channel from the async Gemini task
/// to the UI thread for display updates.
pub(crate) enum StreamEvent {
    /// A chunk of response text arrived.
    Chunk(String),
    /// A chunk of thinking/reasoning text arrived.
    Thought(String),
    /// An error occurred during streaming.
    Error(String),
    /// The stream has completed.
    Done,
}
