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

    let items: Vec<ListItem> = if space_state.users_error {
        vec![ListItem::new("⚠ ユーザー取得失敗")]
    } else {
        let mut items = vec![ListItem::new("ALL (フィルターなし)")];
        if let Some(users) = &space_state.users {
            for user in users {
                items.push(ListItem::new(user.name.clone()));
            }
        }
        items
    };

    let list = List::new(items)
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
    list_state.select(Some(state.filter_cursor_idx));

    frame.render_stateful_widget(list, popup_area, &mut list_state);

    if popup_area.height > 2 {
        let help_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + popup_area.height - 2,
            width: popup_area.width.saturating_sub(2),
            height: 1,
        };
        let help = Paragraph::new("[Enter] 選択  [Esc] キャンセル")
            .style(Style::default().fg(Color::DarkGray));
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
