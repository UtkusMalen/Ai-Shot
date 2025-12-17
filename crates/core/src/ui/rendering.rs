//! UI rendering helpers and overlay components.
//!
//! This module contains reusable rendering functions for the snipping tool UI,
//! including the selection overlay, dark cutout effect, and popup windows.

use eframe::egui;

/// Draws the dark overlay with a transparent "cutout" for the selection area.
///
/// Creates a visual effect where the selected region is clear/bright while
/// the rest of the screen is dimmed, helping users focus on their selection.
///
/// # Arguments
/// * `painter` - The egui painter to draw with
/// * `screen_rect` - The full screen rectangle
/// * `selection_rect` - The selected area to keep clear
/// * `alpha` - Darkness level (0-255, higher = darker)
pub fn draw_selection_overlay(
    painter: &egui::Painter,
    screen_rect: egui::Rect,
    selection_rect: egui::Rect,
    alpha: u8,
) {
    let color = egui::Color32::from_black_alpha(alpha);

    // Top region (above selection)
    painter.rect_filled(
        egui::Rect::from_min_max(
            screen_rect.min,
            egui::pos2(screen_rect.max.x, selection_rect.min.y),
        ),
        0.0,
        color,
    );

    // Bottom region (below selection)
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(screen_rect.min.x, selection_rect.max.y),
            screen_rect.max,
        ),
        0.0,
        color,
    );

    // Left region (left of selection, between top and bottom)
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(screen_rect.min.x, selection_rect.min.y),
            egui::pos2(selection_rect.min.x, selection_rect.max.y),
        ),
        0.0,
        color,
    );

    // Right region (right of selection, between top and bottom)
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(selection_rect.max.x, selection_rect.min.y),
            egui::pos2(screen_rect.max.x, selection_rect.max.y),
        ),
        0.0,
        color,
    );
}

/// Draws a border around the selection rectangle.
///
/// # Arguments
/// * `painter` - The egui painter to draw with
/// * `selection_rect` - The selected area
/// * `stroke_width` - Width of the border line
/// * `color` - Color of the border
pub fn draw_selection_border(
    painter: &egui::Painter,
    selection_rect: egui::Rect,
    stroke_width: f32,
    color: egui::Color32,
) {
    painter.rect_stroke(
        selection_rect,
        0.0,
        egui::Stroke::new(stroke_width, color),
        egui::StrokeKind::Middle,
    );
}

/// Calculates the optimal position for a popup window relative to a selection.
///
/// Tries to position the window below the selection, but moves it above
/// if there isn't enough space below.
///
/// # Arguments
/// * `selection_rect` - The selected area
/// * `screen_rect` - The full screen rectangle  
/// * `window_width` - Desired width of the popup
/// * `spacing` - Gap between selection and popup
/// * `min_space_needed` - Minimum vertical space needed for the popup
///
/// # Returns
/// A tuple of (x position, y position, pivot alignment)
pub fn calculate_popup_position(
    selection_rect: egui::Rect,
    screen_rect: egui::Rect,
    window_width: f32,
    spacing: f32,
    min_space_needed: f32,
) -> (f32, f32, egui::Align2) {
    // Center horizontally relative to selection, clamped to screen
    let window_x = (selection_rect.center().x - (window_width / 2.0))
        .clamp(10.0, screen_rect.width() - window_width - 10.0);

    // Default: position below selection
    let mut window_y = selection_rect.max.y + spacing;
    let mut pivot = egui::Align2::LEFT_TOP;

    // Check if there's enough space below
    let space_below = screen_rect.max.y - window_y;
    if space_below < min_space_needed && selection_rect.min.y > space_below {
        // Not enough space below, but more space above - position above
        pivot = egui::Align2::LEFT_BOTTOM;
        window_y = selection_rect.min.y - spacing;
    }

    (window_x, window_y, pivot)
}
