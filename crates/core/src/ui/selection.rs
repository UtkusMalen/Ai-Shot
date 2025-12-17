//! Selection handling and coordinate mapping.
//!
//! This module contains logic for handling user selection interactions
//! and mapping between UI coordinates and image coordinates.

use eframe::egui;

/// Minimum distance (in pixels) for a drag to be considered a valid selection.
pub const MIN_SELECTION_DISTANCE: f32 = 10.0;

/// Handles selection drag state and returns the appropriate selection rectangle.
///
/// During an active drag, returns the rectangle from start to current pointer position.
/// After a finalized selection, returns the static selection rectangle.
///
/// # Arguments
/// * `selection_start` - The starting position of the selection
/// * `current_pos` - The current/ending position
/// * `is_finalized` - Whether the selection has been finalized
/// * `ctx` - The egui context for getting current pointer position
///
/// # Returns
/// The interaction position to use for drawing the selection
#[allow(dead_code)]
pub fn get_interaction_position(
    _selection_start: Option<egui::Pos2>,
    current_pos: Option<egui::Pos2>,
    is_finalized: bool,
    ctx: &egui::Context,
) -> Option<egui::Pos2> {
    if is_finalized {
        current_pos
    } else {
        ctx.pointer_interact_pos().or(current_pos)
    }
}

/// Determines if a drag operation should be considered a valid selection.
///
/// A selection is valid if the start and end points are far enough apart
/// to indicate intentional selection rather than an accidental click.
///
/// # Arguments
/// * `start` - Starting position of the drag
/// * `end` - Ending position of the drag
pub fn is_valid_selection(start: egui::Pos2, end: egui::Pos2) -> bool {
    start.distance(end) > MIN_SELECTION_DISTANCE
}

/// Normalizes a selection rectangle to ensure positive width/height.
///
/// Users can drag in any direction, which may result in negative-sized
/// rectangles. This function ensures the rectangle has positive dimensions.
///
/// # Arguments
/// * `start` - One corner of the selection
/// * `end` - The opposite corner of the selection
#[allow(dead_code)]
pub fn normalize_selection(start: egui::Pos2, end: egui::Pos2) -> egui::Rect {
    egui::Rect::from_two_pos(start, end)
}

/// Result of processing selection input events.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionEvent {
    /// User started a new selection drag.
    Started,
    /// User is actively dragging.
    Dragging,
    /// User completed a valid selection.
    Completed,
    /// User completed a drag but it was too small/invalid.
    Cancelled,
    /// No selection event occurred.
    None,
}

/// Processes drag events and returns the selection state change.
///
/// # Arguments
/// * `response` - The egui response from the interaction area
/// * `start` - Current selection start position (mutable)
/// * `current` - Current selection end position (mutable)
/// * `is_finalized` - Current finalized state
pub fn process_drag_event(
    response: &egui::Response,
    start: &mut Option<egui::Pos2>,
    current: &mut Option<egui::Pos2>,
    is_finalized: bool,
) -> SelectionEvent {
    if response.drag_started() {
        *start = response.interact_pointer_pos();
        *current = response.interact_pointer_pos();
        return SelectionEvent::Started;
    }

    if response.dragged() {
        *current = response.interact_pointer_pos();
        return SelectionEvent::Dragging;
    }

    if response.drag_stopped() && !is_finalized {
        if let (Some(s), Some(e)) = (*start, *current) {
            if is_valid_selection(s, e) {
                return SelectionEvent::Completed;
            } else {
                *start = None;
                *current = None;
                return SelectionEvent::Cancelled;
            }
        }
    }

    SelectionEvent::None
}
