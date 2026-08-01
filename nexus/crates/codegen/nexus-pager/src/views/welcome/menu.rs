//! Menu component — renders shortcut key menus.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::theme::Theme;

use super::logo::logo_visual_width;

/// Render the welcome menu rows as `label … shortcut`, padded within each row.
/// Returns the Rect for each item row (for hit-testing clicks and hover).
pub fn render_menu(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    items: &[(&str, &str)],
    selected: Option<usize>,
    mouse_pos: Option<(u16, u16)>,
    min_width_hint: u16,
) -> Vec<Rect> {
    let label_style = Style::default()
        .fg(theme.text_primary)
        .add_modifier(Modifier::BOLD);
    let label_selected_style = Style::default()
        .fg(theme.text_primary)
        .bg(theme.bg_highlight)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(theme.gray_bright);
    let key_selected_style = Style::default()
        .fg(theme.gray_bright)
        .bg(theme.bg_highlight);

    // Width: label + gap + key. Keep a 4-col gap between label and key for
    // readability.
    let content_min: u16 = items
        .iter()
        .map(|(key, label)| (key.len() + label.len() + 4) as u16)
        .max()
        .unwrap_or(0);
    let menu_width = logo_visual_width(area.height)
        .max(30)
        .max(content_min)
        .max(min_width_hint)
        // Clamp to the available width so rows never exceed the window and
        // get soft-wrapped by the terminal on narrow terminals (which garbles
        // the label/key alignment). Wide windows are unaffected — the logo
        // drives the width there and stays centered.
        .min(area.width.max(1));

    let [_, menu_centered, _] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(menu_width),
        Constraint::Min(0),
    ])
    .flex(Flex::Center)
    .areas(area);

    let mut rects = Vec::with_capacity(items.len());
    let mut y = menu_centered.y;
    for (i, (key, label)) in items.iter().enumerate() {
        if y >= menu_centered.y + menu_centered.height {
            break;
        }

        let is_selected = selected == Some(i);
        let key_width = key.len() as u16;
        let label_len = label.len() as u16;

        let row_rect = Rect {
            x: menu_centered.x,
            y,
            width: menu_centered.width,
            height: 1,
        };
        rects.push(row_rect);

        // Fill row background when selected/hovered
        if is_selected {
            let hover_bg = Style::default().bg(theme.bg_highlight);
            for x in menu_centered.x..menu_centered.x + menu_centered.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(hover_bg);
                }
            }
        }

        // Label, flush with the left edge of the menu column.
        let lstyle = if is_selected {
            label_selected_style
        } else {
            label_style
        };
        buf.set_span(menu_centered.x, y, &Span::styled(*label, lstyle), label_len);

        // Key shortcut flush with the right edge of the menu column.
        let kstyle = if is_selected {
            key_selected_style
        } else {
            key_style
        };
        buf.set_span(
            menu_centered.x + menu_centered.width - key_width,
            y,
            &Span::styled(*key, kstyle),
            key_width,
        );

        // [x] dismiss affordance restyling (for the import row)
        if let Some(x_offset) = key.rfind("[x]") {
            let key_x_start = menu_centered.x + menu_centered.width - key_width;
            let dismiss_start = key_x_start + x_offset as u16;
            let dismiss_end = dismiss_start + 3;
            let mouse_on_dismiss = mouse_pos
                .is_some_and(|(mx, my)| my == y && mx >= dismiss_start && mx < dismiss_end);
            let dismiss_color = if mouse_on_dismiss {
                theme.text_primary
            } else {
                theme.gray_bright
            };
            let dismiss_style = if is_selected {
                Style::default()
                    .fg(dismiss_color)
                    .bg(theme.bg_highlight)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(dismiss_color)
                    .add_modifier(Modifier::BOLD)
            };
            for (offset, ch) in "[x]".chars().enumerate() {
                let col = dismiss_start + offset as u16;
                if let Some(cell) = buf.cell_mut((col, y)) {
                    cell.set_char(ch);
                    cell.set_style(dismiss_style);
                }
            }
        }

        y += 1;
    }

    rects
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On a narrow terminal the menu column must clamp to the available
    /// width instead of overflowing — an overflowing row gets soft-wrapped
    /// by the terminal, garbling the label/key alignment. The unclamped
    /// width (driven by the ASCII logo, ~100 cols) would overflow at 20/40.
    #[test]
    fn narrow_areas_clamp_menu_width_to_area() {
        let theme = Theme::current();
        // (key shortcut, label) — render_menu's item contract.
        let items: &[(&str, &str)] = &[
            ("l", "Login with sage.local"),
            ("k", "Set API key"),
            ("q", "Quit"),
        ];
        for width in [20u16, 40, 80] {
            let area = Rect {
                x: 0,
                y: 0,
                width,
                height: 40,
            };
            let mut buf = Buffer::empty(area);
            let rects = render_menu(area, &mut buf, &theme, items, None, None, 0);
            assert_eq!(rects.len(), items.len());
            for (i, r) in rects.iter().enumerate() {
                assert!(
                    r.width <= area.width,
                    "width {width}: menu row {i} must be clamped to the area (got width {})",
                    r.width
                );
                assert!(
                    r.x + r.width <= area.x + area.width,
                    "width {width}: menu row {i} must stay inside the area"
                );
            }
        }
    }

    /// Each menu row renders the label flush-left and the key shortcut
    /// flush-right within the row.
    #[test]
    fn menu_rows_render_label_left_key_right() {
        let theme = Theme::current();
        // (key shortcut, label) — render_menu's item contract.
        let items: &[(&str, &str)] = &[
            ("l", "Login with sage.local"),
            ("k", "Set API key"),
            ("q", "Quit"),
        ];
        let area = Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 40,
        };
        let mut buf = Buffer::empty(area);
        let rects = render_menu(area, &mut buf, &theme, items, None, None, 0);
        for (i, (key, label)) in items.iter().enumerate() {
            let row = rects[i];
            let first = buf.cell((row.x, row.y)).unwrap().symbol().to_string();
            assert_eq!(
                first,
                label.chars().next().unwrap().to_string(),
                "row {i}: label must be flush-left"
            );
            let last_x = row.x + row.width - 1;
            let last = buf.cell((last_x, row.y)).unwrap().symbol().to_string();
            assert_eq!(last, key.to_string(), "row {i}: key must be flush-right");
        }
    }
}
