use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};
use ratatui::layout::Rect;
use std::time::Instant;

/// Mouse interaction state and event handling
#[derive(Debug, Clone)]
pub struct MouseState {
    /// Current mouse position (column, row)
    pub position: (u16, u16),
    /// Whether the mouse is currently hovering over a selectable item
    pub hovering: bool,
    /// Index of the item being hovered over (if any)
    pub hover_index: Option<usize>,
    /// Last click position, button, and time
    pub last_click: Option<(u16, u16, MouseButton, Instant)>,
}

impl MouseState {
    pub fn new() -> Self {
        Self {
            position: (0, 0),
            hovering: false,
            hover_index: None,
            last_click: None,
        }
    }

    /// Update mouse position from a mouse event
    pub fn update_position(&mut self, event: &MouseEvent) {
        self.position = (event.column, event.row);
    }

    /// Check if a position is within a rectangular area
    pub fn is_in_area(&self, area: Rect) -> bool {
        let (col, row) = self.position;
        col >= area.x
            && col < area.x + area.width
            && row >= area.y
            && row < area.y + area.height
    }

    /// Get the row index relative to an area's top
    pub fn row_in_area(&self, area: Rect) -> Option<usize> {
        if self.is_in_area(area) {
            Some((self.position.1 - area.y) as usize)
        } else {
            None
        }
    }

    /// Handle a mouse event and return the action to take
    /// Supports multiple UI regions: input, filters, and results
    pub fn handle_event(
        &mut self,
        event: MouseEvent,
        input_area: Rect,
        filters_area: Rect,
        result_area: Rect,
    ) -> MouseAction {
        self.update_position(&event);

        match event.kind {
            MouseEventKind::Down(button) => {
                if button != MouseButton::Left {
                    return MouseAction::None;
                }

                let now = Instant::now();
                let current_pos = (event.column, event.row);

                // Check for double-click (within 300ms at same position)
                let is_double_click = if let Some((last_col, last_row, last_button, last_time)) = self.last_click {
                    last_button == MouseButton::Left
                        && last_col == current_pos.0
                        && last_row == current_pos.1
                        && now.duration_since(last_time).as_millis() < 300
                } else {
                    false
                };

                // Update last click
                self.last_click = Some((event.column, event.row, button, now));

                // Check input area (click to focus)
                if self.is_in_area(input_area) {
                    // Calculate cursor position (subtract 1 for left border)
                    let cursor_pos = (event.column.saturating_sub(input_area.x + 1)) as usize;
                    return MouseAction::FocusInput(cursor_pos);
                }

                // Check filters area (click to toggle filters)
                if self.is_in_area(filters_area) {
                    let col = event.column.saturating_sub(filters_area.x + 1);

                    // Symbols badge is at positions 0-14: " [s] Symbols "
                    if col < 14 {
                        return MouseAction::ToggleSymbols;
                    }
                    // Regex badge is at positions 16-28: " [r] Regex "
                    if col >= 16 && col < 28 {
                        return MouseAction::ToggleRegex;
                    }
                    return MouseAction::None;
                }

                // Check results area (click to select)
                if self.is_in_area(result_area) {
                    if let Some(row) = self.row_in_area(result_area) {
                        // Subtract 1 to account for top border of the List widget
                        if row > 0 {
                            let content_row = row - 1;
                            return if is_double_click {
                                MouseAction::DoubleClick(content_row)
                            } else {
                                MouseAction::SelectResult(content_row)
                            };
                        }
                    }
                }

                MouseAction::None
            }
            MouseEventKind::ScrollDown => {
                if self.is_in_area(result_area) {
                    MouseAction::ScrollDown
                } else {
                    MouseAction::None
                }
            }
            MouseEventKind::ScrollUp => {
                if self.is_in_area(result_area) {
                    MouseAction::ScrollUp
                } else {
                    MouseAction::None
                }
            }
            MouseEventKind::Moved => {
                if self.is_in_area(result_area) {
                    if let Some(row) = self.row_in_area(result_area) {
                        self.hovering = true;
                        self.hover_index = Some(row);
                        return MouseAction::Hover(row);
                    }
                }
                self.hovering = false;
                self.hover_index = None;
                MouseAction::None
            }
            _ => MouseAction::None,
        }
    }
}

/// Actions triggered by mouse events
#[derive(Debug, Clone, PartialEq)]
pub enum MouseAction {
    /// No action
    None,
    /// Select a result at the given index
    SelectResult(usize),
    /// Double-click on a result at the given index
    DoubleClick(usize),
    /// Hover over a result at the given index
    Hover(usize),
    /// Scroll down
    ScrollDown,
    /// Scroll up
    ScrollUp,
    /// Click on input field to focus (cursor position)
    FocusInput(usize),
    /// Toggle symbols filter
    ToggleSymbols,
    /// Toggle regex filter
    ToggleRegex,
    /// Close file preview
    ClosePreview,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_state_creation() {
        let state = MouseState::new();
        assert_eq!(state.position, (0, 0));
        assert!(!state.hovering);
    }

    #[test]
    fn test_is_in_area() {
        let mut state = MouseState::new();
        state.position = (10, 5);

        let area = Rect::new(5, 3, 20, 10);
        assert!(state.is_in_area(area));

        state.position = (30, 5);
        assert!(!state.is_in_area(area));
    }

    #[test]
    fn test_row_in_area() {
        let mut state = MouseState::new();
        state.position = (10, 8);

        let area = Rect::new(5, 5, 20, 10);
        assert_eq!(state.row_in_area(area), Some(3));

        state.position = (10, 20);
        assert_eq!(state.row_in_area(area), None);
    }
}
