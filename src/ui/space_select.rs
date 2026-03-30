use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // help bar
        ])
        .split(area);

    render_title(frame, chunks[0]);
    render_content(frame, chunks[1], state);
    render_help_bar(frame, chunks[2]);
}

fn render_title(frame: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new(" lazybacklog").style(
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}

fn render_content(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Spaces ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = state
        .config
        .spaces
        .iter()
        .map(|s| ListItem::new(s.name.clone()))
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.space_cursor_idx));

    frame.render_stateful_widget(list, inner, &mut list_state);
}

fn render_help_bar(frame: &mut Frame, area: Rect) {
    let text = " [j/k] Move  [Enter] Select  [q] Quit";
    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, SpaceConfig};
    use ratatui::{backend::TestBackend, Terminal};

    fn make_state() -> AppState {
        let config = Config {
            default_space: "space1".to_string(),
            spaces: vec![SpaceConfig {
                name: "space1".to_string(),
                host: "space1.backlog.com".to_string(),
                api_key: "key".to_string(),
            }],
        };
        AppState::new(config, false)
    }

    #[test]
    fn space_select_content_has_border_title() {
        // The rendered output should contain the "Spaces" title text
        // We verify by checking that "Spaces" appears in the rendered buffer
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let state = make_state();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render(frame, area, &state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

        assert!(
            content.contains("Spaces"),
            "Expected 'Spaces' title in rendered output, got: {:?}",
            content
        );
    }
}
