use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};
use ratatui::layout::Rect;

/// Mouse interaction state and event handling
#[derive(Debug, Clone)]
pub struct MouseState {
    /// Current mouse position (column, row)
    pub position: (u16, u16),
    /// Whether the mouse is currently hovering over a selectable item
    pub hovering: bool,
    /// Index of the item being hovered over (if any)
    pub hover_index: Option<usize>,
    /// Last click position and button
    pub last_click: Option<(u16, u16, MouseButton)>,
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
    pub fn handle_event(&mut self, event: MouseEvent, result_area: Rect) -> MouseAction {
        self.update_position(&event);

        match event.kind {
            MouseEventKind::Down(button) => {
                self.last_click = Some((event.column, event.row, button));

                if self.is_in_area(result_area) {
                    if let Some(row) = self.row_in_area(result_area) {
                        return match button {
                            MouseButton::Left => MouseAction::SelectResult(row),
                            _ => MouseAction::None,
                        };
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
    /// Hover over a result at the given index
    Hover(usize),
    /// Scroll down
    ScrollDown,
    /// Scroll up
    ScrollUp,
    /// Click on a filter badge
    ToggleFilter(String),
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
