use crate::api::models::IssueStatus;
use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup_area = centered_rect(50, 60, area);
    frame.render_widget(Clear, popup_area);

    let space_state = state.current_space_state();

    let items: Vec<ListItem> = match &space_state.statuses {
        None => vec![ListItem::new("読み込み中...")],
        Some(statuses) if statuses.is_empty() => vec![ListItem::new("ステータスなし")],
        Some(statuses) => statuses
            .iter()
            .map(|s| {
                let checked = state.status_filter_pending.contains(&s.id);
                let checkbox = if checked { "[✓]" } else { "[ ]" };
                ListItem::new(Line::from(vec![Span::raw(format!(
                    "{} {}",
                    checkbox, s.name
                ))]))
            })
            .collect(),
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(" ステータスフィルター ")
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
    // Only show cursor when statuses are actually loaded
    if space_state
        .statuses
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
    {
        list_state.select(Some(state.status_filter_cursor_idx));
    }

    frame.render_stateful_widget(list, popup_area, &mut list_state);

    if popup_area.height > 2 {
        let help_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + popup_area.height - 2,
            width: popup_area.width.saturating_sub(2),
            height: 1,
        };
        let help = Paragraph::new("[j/k] 移動  [Space] 切替  [Enter] 決定  [Esc] キャンセル")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, help_area);
    }
}

/// Generate the filter bar status text from current filter state.
/// Returns "ALL", "(なし)", or comma-separated status names.
pub fn status_filter_text(filter_ids: &[i64], statuses: &Option<Vec<IssueStatus>>) -> String {
    let Some(statuses) = statuses else {
        return "読み込み中...".to_string();
    };
    if statuses.is_empty() {
        return "ALL".to_string();
    }
    if filter_ids.is_empty() {
        return "(なし)".to_string();
    }
    if filter_ids.len() == statuses.len() {
        return "ALL".to_string();
    }
    statuses
        .iter()
        .filter(|s| filter_ids.contains(&s.id))
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Toggle a status ID in the pending list. If present, removes it; if absent, appends it.
pub fn toggle_status(pending: &mut Vec<i64>, id: i64) {
    if let Some(pos) = pending.iter().position(|&x| x == id) {
        pending.remove(pos);
    } else {
        pending.push(id);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_statuses() -> Vec<IssueStatus> {
        vec![
            IssueStatus {
                id: 1,
                name: "未対応".to_string(),
            },
            IssueStatus {
                id: 2,
                name: "処理中".to_string(),
            },
            IssueStatus {
                id: 3,
                name: "処理済み".to_string(),
            },
            IssueStatus {
                id: 4,
                name: "完了".to_string(),
            },
        ]
    }

    #[test]
    fn test_status_filter_text_all_selected() {
        let statuses = make_statuses();
        let ids = vec![1, 2, 3, 4];
        assert_eq!(status_filter_text(&ids, &Some(statuses)), "ALL");
    }

    #[test]
    fn test_status_filter_text_none_selected() {
        let statuses = make_statuses();
        assert_eq!(status_filter_text(&[], &Some(statuses)), "(なし)");
    }

    #[test]
    fn test_status_filter_text_partial() {
        let statuses = make_statuses();
        let ids = vec![1, 2];
        assert_eq!(status_filter_text(&ids, &Some(statuses)), "未対応, 処理中");
    }

    #[test]
    fn test_status_filter_text_loading() {
        assert_eq!(status_filter_text(&[], &None), "読み込み中...");
    }

    #[test]
    fn test_status_filter_text_empty_statuses_all() {
        assert_eq!(status_filter_text(&[], &Some(vec![])), "ALL");
    }

    #[test]
    fn test_toggle_status_add() {
        let mut pending = vec![1i64, 3];
        toggle_status(&mut pending, 2);
        assert_eq!(pending, vec![1, 3, 2]);
    }

    #[test]
    fn test_toggle_status_remove() {
        let mut pending = vec![1i64, 2, 3];
        toggle_status(&mut pending, 2);
        assert_eq!(pending, vec![1, 3]);
    }

    #[test]
    fn test_toggle_status_add_to_empty() {
        let mut pending: Vec<i64> = vec![];
        toggle_status(&mut pending, 5);
        assert_eq!(pending, vec![5]);
    }
}
