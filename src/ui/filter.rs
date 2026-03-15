use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup_area = centered_rect(40, 60, area);
    frame.render_widget(Clear, popup_area);

    let space_state = state.current_space_state();

    if space_state.users_error {
        let list = List::new(vec![ListItem::new("⚠ ユーザー取得失敗")]).block(
            Block::default()
                .title(" Assigneeフィルター ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(list, popup_area);
        return;
    }

    // Build full item list: [(full_list_idx, display_text)]
    let mut all_items: Vec<(usize, String)> = vec![(0, "ALL (フィルターなし)".to_string())];
    if let Some(users) = &space_state.users {
        for (i, user) in users.iter().enumerate() {
            all_items.push((i + 1, user.name.clone()));
        }
    }

    // Filter by search query
    let display_items: Vec<(usize, &str)> = if state.search_query.is_empty() {
        all_items.iter().map(|(i, s)| (*i, s.as_str())).collect()
    } else {
        let match_indices = state.matching_user_indices();
        all_items
            .iter()
            .filter(|(i, _)| match_indices.contains(i))
            .map(|(i, s)| (*i, s.as_str()))
            .collect()
    };

    // Position of filter_cursor_idx within displayed items
    let display_selected = display_items
        .iter()
        .position(|(i, _)| *i == state.filter_cursor_idx);

    let list_items: Vec<ListItem> = display_items
        .iter()
        .map(|(_, text)| ListItem::new(*text))
        .collect();

    // Reserve bottom line for help/search bar (2 lines from bottom = inside border)
    let list = List::new(list_items)
        .block(
            Block::default()
                .title(" Assigneeフィルター ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    if let Some(pos) = display_selected {
        list_state.select(Some(pos));
    }

    frame.render_stateful_widget(list, popup_area, &mut list_state);

    if popup_area.height > 2 {
        let help_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + popup_area.height - 2,
            width: popup_area.width.saturating_sub(2),
            height: 1,
        };
        let help_text = if state.search_active {
            format!("/ {}█  ({} matches)", state.search_query, display_items.len())
        } else if !state.search_query.is_empty() {
            format!(
                "/ {}  ({} matches)  [n/N] 移動  [Esc] 解除",
                state.search_query,
                display_items.len()
            )
        } else {
            "[Enter] 選択  [/] 検索  [Esc] キャンセル".to_string()
        };
        let help_style = if state.search_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let help = Paragraph::new(help_text).style(help_style);
        frame.render_widget(help, help_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let width = (area.width as u32 * percent_x as u32 / 100) as u16;
    let height = (area.height as u32 * percent_y as u32 / 100) as u16;
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}
