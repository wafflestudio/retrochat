use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Creates a centered rectangle within a given area
///
/// # Arguments
/// * `percent_x` - Horizontal percentage of the area to use (0-100)
/// * `percent_y` - Vertical percentage of the area to use (0-100)
/// * `r` - The area to center within
///
/// # Returns
/// A `Rect` that is centered within the given area
///
/// # Example
/// ```
/// use retrochat_tui::utils::layout::centered_rect;
/// use ratatui::layout::Rect;
///
/// let frame_area = Rect::new(0, 0, 100, 100);
/// let popup = centered_rect(60, 40, frame_area);
/// assert_eq!(popup.width, 60);
/// assert_eq!(popup.height, 40);
/// ```
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
