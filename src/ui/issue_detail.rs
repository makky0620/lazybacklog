use crate::api::models::{Comment, Issue};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    issue: &Issue,
    scroll_offset: u16,
    comments: Option<&[Comment]>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // details block (3 content lines + 2 border)
            Constraint::Min(0),    // description + comments block
            Constraint::Length(1), // help bar
        ])
        .split(area);

    render_details(frame, chunks[0], issue);
    render_description_and_comments(frame, chunks[1], issue, scroll_offset, comments);
    render_help_bar(frame, chunks[2]);
}

fn render_details(frame: &mut Frame, area: Rect, issue: &Issue) {
    let assignee = issue
        .assignee
        .as_ref()
        .map(|u| u.name.as_str())
        .unwrap_or("-");
    let priority = issue
        .priority
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("-");
    let issue_type = issue
        .issue_type
        .as_ref()
        .map(|t| t.name.as_str())
        .unwrap_or("-");
    let due_date = issue.due_date.as_deref().unwrap_or("-");

    let lines = vec![
        Line::from(vec![
            Span::styled("Assignee: ", Style::default().fg(Color::Cyan)),
            Span::raw(assignee.to_string()),
            Span::raw("    "),
            Span::styled("Priority: ", Style::default().fg(Color::Cyan)),
            Span::raw(priority.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Status:   ", Style::default().fg(Color::Cyan)),
            Span::raw(issue.status.name.clone()),
            Span::raw("    "),
            Span::styled("Type:     ", Style::default().fg(Color::Cyan)),
            Span::raw(issue_type.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Due:      ", Style::default().fg(Color::Cyan)),
            Span::raw(due_date.to_string()),
        ]),
    ];

    let block = Block::default()
        .title(format!(" {}: {} ", issue.issue_key, issue.summary))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_description_and_comments(
    frame: &mut Frame,
    area: Rect,
    issue: &Issue,
    scroll_offset: u16,
    comments: Option<&[Comment]>,
) {
    let description = issue.description.as_deref().unwrap_or("(no description)");
    let mut lines: Vec<Line> = description
        .lines()
        .map(|l| Line::from(l.to_string()))
        .collect();

    match comments {
        None => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "(loading comments...)",
                Style::default().fg(Color::DarkGray),
            )));
        }
        Some(comments) => {
            for (i, comment) in comments.iter().enumerate() {
                let author = comment
                    .created_user
                    .as_ref()
                    .map(|u| u.name.as_str())
                    .unwrap_or("?");
                let date = comment.created.get(..10).unwrap_or(&comment.created);
                let separator = format!("── {}: {}  {} ──", i + 1, author, date);
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    separator,
                    Style::default().fg(Color::DarkGray),
                )));
                let content = comment.content.as_deref().unwrap_or("");
                for cl in content.lines() {
                    lines.push(Line::from(cl.to_string()));
                }
            }
        }
    }

    let block = Block::default()
        .title(" Description & Comments ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));
    frame.render_widget(paragraph, area);
}

fn render_help_bar(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new(" [j/k] Scroll  [o] Open  [Esc] Back")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::{Issue, IssueStatus};
    use ratatui::{backend::TestBackend, Terminal};

    fn make_issue() -> Issue {
        Issue {
            id: 1,
            issue_key: "PROJ-1".to_string(),
            summary: "Fix login bug".to_string(),
            description: Some("Login fails when entering email.".to_string()),
            assignee: None,
            status: IssueStatus {
                id: 1,
                name: "Open".to_string(),
            },
            priority: None,
            issue_type: None,
            due_date: None,
        }
    }

    #[test]
    fn issue_detail_shows_issue_key_in_details_title() {
        let backend = TestBackend::new(60, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let issue = make_issue();
        terminal
            .draw(|frame| render(frame, frame.area(), &issue, 0, None))
            .unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            content.contains("PROJ-1"),
            "Expected issue key in rendered output, got: {:?}",
            content
        );
    }

    #[test]
    fn issue_detail_shows_description_and_comments_block_title() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let issue = make_issue();
        terminal
            .draw(|frame| render(frame, frame.area(), &issue, 0, None))
            .unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            content.contains("Description"),
            "Expected 'Description' in block title, got: {:?}",
            content
        );
    }

    #[test]
    fn issue_detail_shows_loading_when_comments_none() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let issue = make_issue();
        terminal
            .draw(|frame| render(frame, frame.area(), &issue, 0, None))
            .unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            content.contains("loading comments"),
            "Expected loading indicator, got: {:?}",
            content
        );
    }

    #[test]
    fn issue_detail_shows_comment_content() {
        let backend = TestBackend::new(60, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        let issue = make_issue();
        let comments = vec![Comment {
            id: 1,
            content: Some("Great bug report".to_string()),
            created_user: Some(crate::api::models::User {
                id: 10,
                name: "Alice".to_string(),
            }),
            created: "2026-03-31T12:00:00Z".to_string(),
        }];
        terminal
            .draw(|frame| render(frame, frame.area(), &issue, 0, Some(&comments)))
            .unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            content.contains("Great bug report"),
            "Expected comment content, got: {:?}",
            content
        );
    }

    #[test]
    fn issue_detail_no_cyan_title_bar_at_top() {
        let backend = TestBackend::new(60, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let issue = make_issue();
        terminal
            .draw(|frame| render(frame, frame.area(), &issue, 0, None))
            .unwrap();
        let first_symbol = terminal.backend().buffer().content()[0]
            .symbol()
            .to_string();
        assert_eq!(
            first_symbol, "┌",
            "Expected border char at top-left (no title bar), got: {:?}",
            first_symbol
        );
    }
}
