// NOTE: Do NOT import `crossterm::event` with `{self, ...}` — the local `mod event`
// module creates an E0255 name conflict. Import specific types only.
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use tokio::sync::mpsc;

use crate::api;
use crate::app::{AppState, Screen};
use crate::config;
use crate::event::AppEvent;
use crate::mock;
use crate::ui; // needed for ui::status_filter::toggle_status

pub fn handle_list_key(
    key: KeyEvent,
    state: &mut AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    // Search mode: intercept all keys for query editing
    if state.search_active {
        match key.code {
            KeyCode::Char(c) => {
                state.search_query.push(c);
                state.search_match_idx = 0;
                let matches = state.matching_issue_indices();
                if let Some(&first) = matches.first() {
                    state.selected_issue_idx = first;
                }
            }
            KeyCode::Backspace => {
                state.search_query.pop();
                if !state.search_query.is_empty() {
                    state.search_match_idx = 0;
                    let matches = state.matching_issue_indices();
                    if let Some(&first) = matches.first() {
                        state.selected_issue_idx = first;
                    }
                }
            }
            KeyCode::Enter => {
                state.search_active = false;
            }
            KeyCode::Esc => {
                state.clear_search();
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Char('q') => state.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => state.navigate_down(),
        KeyCode::Char('k') | KeyCode::Up => state.navigate_up(),
        KeyCode::Enter => {
            if state.demo_mode {
                if let Some(issue) = state.selected_issue().cloned() {
                    let _ = tx.send(AppEvent::IssueDetailLoaded(issue));
                }
                return;
            }
            if let Some(issue) = state.selected_issue() {
                let issue_key = issue.issue_key.clone();
                let space_name = state.current_space_name().to_string();
                let space_cfg = config
                    .spaces
                    .iter()
                    .find(|s| s.name == space_name)
                    .unwrap()
                    .clone();
                let tx = tx.clone();
                tokio::spawn(async move {
                    match api::client::BacklogClient::new(space_cfg.host, space_cfg.api_key) {
                        Ok(client) => match client.fetch_issue(&issue_key).await {
                            Ok(issue) => {
                                let _ = tx.send(AppEvent::IssueDetailLoaded(issue));
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::ApiError {
                                    space: space_name,
                                    message: e.to_string(),
                                });
                            }
                        },
                        Err(e) => {
                            let _ = tx.send(AppEvent::ApiError {
                                space: space_name,
                                message: e.to_string(),
                            });
                        }
                    }
                });
            }
        }
        KeyCode::Char('f') => {
            let assignee_id = state.filter_assignee_id;
            state.filter_cursor_idx = if assignee_id.is_none() {
                0
            } else {
                state
                    .current_space_state()
                    .users
                    .as_ref()
                    .and_then(|users| {
                        users
                            .iter()
                            .position(|u| Some(u.id) == assignee_id)
                            .map(|i| i + 1)
                    })
                    .unwrap_or(0)
            };
            state.screen = Screen::Filter;
        }
        KeyCode::Char('s') => {
            state.status_filter_pending = state.current_space_state().filter_status_ids.clone();
            state.status_filter_cursor_idx = 0;
            state.screen = Screen::StatusFilter;
        }
        KeyCode::Esc => {
            state.selected_issue_idx = 0;
            state.clear_search();
            state.screen = Screen::ProjectSelect;
        }
        KeyCode::Char('r') => {
            state.clear_search();
            let project_id = state.selected_project().map(|p| p.id);
            let assignee_id = state.filter_assignee_id;
            let status_ids = state.current_space_state().filter_status_ids.clone();
            state.current_space_state_mut().issues = None;
            state.current_space_state_mut().loading_issues = true;
            fetch_issues(state, config, tx, project_id, assignee_id, status_ids);
        }
        KeyCode::Char('/') => {
            state.search_active = true;
            state.search_query.clear();
            state.search_match_idx = 0;
        }
        KeyCode::Char('n') => {
            if !state.search_query.is_empty() {
                let matches = state.matching_issue_indices();
                if !matches.is_empty() {
                    state.search_match_idx = (state.search_match_idx + 1) % matches.len();
                    state.selected_issue_idx = matches[state.search_match_idx];
                }
            }
        }
        KeyCode::Char('N') => {
            if !state.search_query.is_empty() {
                let matches = state.matching_issue_indices();
                if !matches.is_empty() {
                    state.search_match_idx =
                        (state.search_match_idx + matches.len() - 1) % matches.len();
                    state.selected_issue_idx = matches[state.search_match_idx];
                }
            }
        }
        _ => {}
    }
}

pub fn handle_detail_key(key: KeyEvent, state: &mut AppState) {
    if key.code == KeyCode::Esc {
        state.screen = Screen::IssueList;
        state.detail_issue = None;
    }
}

pub fn handle_filter_key(
    key: KeyEvent,
    state: &mut AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let user_count = state
        .current_space_state()
        .users
        .as_ref()
        .map(|u| u.len())
        .unwrap_or(0);
    let total = user_count + 1; // +1 for "ALL"

    // Search mode: intercept all keys for query editing
    if state.search_active {
        match key.code {
            KeyCode::Char(c) => {
                state.search_query.push(c);
                state.search_match_idx = 0;
                let matches = state.matching_user_indices();
                if let Some(&first) = matches.first() {
                    state.filter_cursor_idx = first;
                }
            }
            KeyCode::Backspace => {
                state.search_query.pop();
                if !state.search_query.is_empty() {
                    state.search_match_idx = 0;
                    let matches = state.matching_user_indices();
                    if let Some(&first) = matches.first() {
                        state.filter_cursor_idx = first;
                    }
                }
            }
            KeyCode::Enter => {
                state.search_active = false;
            }
            KeyCode::Esc => {
                state.clear_search();
                state.screen = Screen::IssueList;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc => state.screen = Screen::IssueList,
        KeyCode::Char('j') | KeyCode::Down => {
            if !state.search_query.is_empty() {
                let matches = state.matching_user_indices();
                if let Some(pos) = matches.iter().position(|&i| i > state.filter_cursor_idx) {
                    state.filter_cursor_idx = matches[pos];
                    state.search_match_idx = pos;
                }
            } else if state.filter_cursor_idx + 1 < total {
                state.filter_cursor_idx += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !state.search_query.is_empty() {
                let matches = state.matching_user_indices();
                if let Some(pos) = matches.iter().rposition(|&i| i < state.filter_cursor_idx) {
                    state.filter_cursor_idx = matches[pos];
                    state.search_match_idx = pos;
                }
            } else if state.filter_cursor_idx > 0 {
                state.filter_cursor_idx -= 1;
            }
        }
        KeyCode::Enter => {
            if state.filter_cursor_idx == 0 {
                state.filter_assignee_id = None;
            } else {
                let users = state.current_space_state().users.clone();
                if let Some(users) = users {
                    if let Some(user) = users.get(state.filter_cursor_idx - 1) {
                        state.filter_assignee_id = Some(user.id);
                    }
                }
            }
            state.clear_search();
            state.screen = Screen::IssueList;
            let project_id = state.selected_project().map(|p| p.id);
            let assignee_id = state.filter_assignee_id;
            let status_ids = state.current_space_state().filter_status_ids.clone();
            state.current_space_state_mut().issues = None;
            state.current_space_state_mut().loading_issues = true;
            fetch_issues(state, config, tx, project_id, assignee_id, status_ids);
        }
        KeyCode::Char('/') => {
            state.search_active = true;
            state.search_query.clear();
            state.search_match_idx = 0;
        }
        KeyCode::Char('n') => {
            if !state.search_query.is_empty() {
                let matches = state.matching_user_indices();
                if !matches.is_empty() {
                    state.search_match_idx = (state.search_match_idx + 1) % matches.len();
                    state.filter_cursor_idx = matches[state.search_match_idx];
                }
            }
        }
        KeyCode::Char('N') => {
            if !state.search_query.is_empty() {
                let matches = state.matching_user_indices();
                if !matches.is_empty() {
                    state.search_match_idx =
                        (state.search_match_idx + matches.len() - 1) % matches.len();
                    state.filter_cursor_idx = matches[state.search_match_idx];
                }
            }
        }
        _ => {}
    }
}

pub fn handle_status_filter_key(
    key: KeyEvent,
    state: &mut AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let status_count = state
        .current_space_state()
        .statuses
        .as_ref()
        .map(|s| s.len())
        .unwrap_or(0);

    // Search mode: intercept all keys for query editing
    if state.search_active {
        match key.code {
            KeyCode::Char(c) => {
                state.search_query.push(c);
                state.search_match_idx = 0;
                let matches = state.matching_status_indices();
                if let Some(&first) = matches.first() {
                    state.status_filter_cursor_idx = first;
                }
            }
            KeyCode::Backspace => {
                state.search_query.pop();
                if !state.search_query.is_empty() {
                    state.search_match_idx = 0;
                    let matches = state.matching_status_indices();
                    if let Some(&first) = matches.first() {
                        state.status_filter_cursor_idx = first;
                    }
                }
            }
            KeyCode::Enter => {
                state.search_active = false;
            }
            KeyCode::Esc => {
                state.status_filter_pending = vec![];
                state.clear_search();
                state.screen = Screen::IssueList;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            state.status_filter_pending = vec![];
            state.screen = Screen::IssueList;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !state.search_query.is_empty() {
                let matches = state.matching_status_indices();
                if let Some(pos) = matches
                    .iter()
                    .position(|&i| i > state.status_filter_cursor_idx)
                {
                    state.status_filter_cursor_idx = matches[pos];
                    state.search_match_idx = pos;
                }
            } else if status_count > 0 && state.status_filter_cursor_idx + 1 < status_count {
                state.status_filter_cursor_idx += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !state.search_query.is_empty() {
                let matches = state.matching_status_indices();
                if let Some(pos) = matches
                    .iter()
                    .rposition(|&i| i < state.status_filter_cursor_idx)
                {
                    state.status_filter_cursor_idx = matches[pos];
                    state.search_match_idx = pos;
                }
            } else if state.status_filter_cursor_idx > 0 {
                state.status_filter_cursor_idx -= 1;
            }
        }
        KeyCode::Char(' ') => {
            if status_count > 0 {
                let id = state
                    .current_space_state()
                    .statuses
                    .as_ref()
                    .and_then(|s| s.get(state.status_filter_cursor_idx))
                    .map(|s| s.id);
                if let Some(id) = id {
                    ui::status_filter::toggle_status(&mut state.status_filter_pending, id);
                }
            }
        }
        KeyCode::Enter => {
            let pending = state.status_filter_pending.clone();
            state.current_space_state_mut().filter_status_ids = pending;
            state.status_filter_pending = vec![];
            state.clear_search();
            state.screen = Screen::IssueList;
            let project_id = state.selected_project().map(|p| p.id);
            let assignee_id = state.filter_assignee_id;
            let status_ids = state.current_space_state().filter_status_ids.clone();
            state.current_space_state_mut().issues = None;
            state.current_space_state_mut().loading_issues = true;
            fetch_issues(state, config, tx, project_id, assignee_id, status_ids);
        }
        KeyCode::Char('/') => {
            state.search_active = true;
            state.search_query.clear();
            state.search_match_idx = 0;
        }
        KeyCode::Char('n') => {
            if !state.search_query.is_empty() {
                let matches = state.matching_status_indices();
                if !matches.is_empty() {
                    state.search_match_idx = (state.search_match_idx + 1) % matches.len();
                    state.status_filter_cursor_idx = matches[state.search_match_idx];
                }
            }
        }
        KeyCode::Char('N') => {
            if !state.search_query.is_empty() {
                let matches = state.matching_status_indices();
                if !matches.is_empty() {
                    state.search_match_idx =
                        (state.search_match_idx + matches.len() - 1) % matches.len();
                    state.status_filter_cursor_idx = matches[state.search_match_idx];
                }
            }
        }
        _ => {}
    }
}

pub fn handle_space_select_key(key: KeyEvent, state: &mut AppState, config: &config::Config) {
    match key.code {
        KeyCode::Char('q') => state.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => {
            let max = config.spaces.len().saturating_sub(1);
            if state.space_cursor_idx < max {
                state.space_cursor_idx += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.space_cursor_idx > 0 {
                state.space_cursor_idx -= 1;
            }
        }
        KeyCode::Enter => {
            if config.spaces.is_empty() {
                return;
            }
            let idx = state.space_cursor_idx;
            state.select_space(idx);
        }
        KeyCode::Esc => {} // no-op
        _ => {}
    }
}

pub fn handle_project_select_key(
    key: KeyEvent,
    state: &mut AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let project_count = state
        .current_space_state()
        .projects
        .as_ref()
        .map(|p| p.len())
        .unwrap_or(0);

    match key.code {
        KeyCode::Char('q') => state.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => {
            if project_count > 0 && state.project_cursor_idx + 1 < project_count {
                state.project_cursor_idx += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.project_cursor_idx > 0 {
                state.project_cursor_idx -= 1;
            }
        }
        KeyCode::Esc => {
            state.project_cursor_idx = 0;
            state.screen = Screen::SpaceSelect;
        }
        KeyCode::Enter => {
            if project_count == 0 {
                // No project to select — no-op
                return;
            }
            // Clone the selected project and store it on SpaceState
            let project = state
                .current_space_state()
                .projects
                .as_ref()
                .and_then(|p| p.get(state.project_cursor_idx))
                .cloned();
            if let Some(project) = project {
                let project_id = project.id;
                // Reset status + issue state for new project.
                // Use separate short-lived borrows to avoid borrow conflict.
                state.current_space_state_mut().selected_project = Some(project);
                state.current_space_state_mut().statuses = None;
                state.current_space_state_mut().filter_status_ids = vec![];
                state.current_space_state_mut().issues = None;
                state.current_space_state_mut().loading_statuses = true;
                state.screen = Screen::IssueList;
                // Fetch statuses first; issues fetched automatically after StatusesLoaded
                fetch_statuses(state, config, tx, project_id);
            }
        }
        _ => {}
    }
}

pub fn fetch_issues(
    state: &AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
    project_id: Option<i64>,
    assignee_id: Option<i64>,
    status_ids: Vec<i64>,
) {
    if state.demo_mode {
        let space = state.current_space_name().to_string();
        let _ = tx.send(AppEvent::IssuesLoaded {
            space,
            issues: mock::issues(),
        });
        return;
    }
    let space_name = state.current_space_name().to_string();
    let space_cfg = config
        .spaces
        .iter()
        .find(|s| s.name == space_name)
        .unwrap()
        .clone();
    tokio::spawn(async move {
        match api::client::BacklogClient::new(space_cfg.host, space_cfg.api_key) {
            Ok(client) => match client
                .fetch_issues(project_id, assignee_id, &status_ids)
                .await
            {
                Ok(issues) => {
                    let _ = tx.send(AppEvent::IssuesLoaded {
                        space: space_name,
                        issues,
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::ApiError {
                        space: space_name,
                        message: e.to_string(),
                    });
                }
            },
            Err(e) => {
                let _ = tx.send(AppEvent::ApiError {
                    space: space_name,
                    message: e.to_string(),
                });
            }
        }
    });
}

pub fn fetch_statuses(
    state: &AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
    project_id: i64,
) {
    if state.demo_mode {
        let space = state.current_space_name().to_string();
        let _ = tx.send(AppEvent::StatusesLoaded {
            space,
            statuses: mock::statuses(),
        });
        return;
    }
    let space_name = state.current_space_name().to_string();
    let space_cfg = config
        .spaces
        .iter()
        .find(|s| s.name == space_name)
        .unwrap()
        .clone();
    tokio::spawn(async move {
        match api::client::BacklogClient::new(space_cfg.host, space_cfg.api_key) {
            Ok(client) => match client.fetch_statuses(project_id).await {
                Ok(statuses) => {
                    let _ = tx.send(AppEvent::StatusesLoaded {
                        space: space_name,
                        statuses,
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::ApiError {
                        space: space_name,
                        message: e.to_string(),
                    });
                }
            },
            Err(e) => {
                let _ = tx.send(AppEvent::ApiError {
                    space: space_name,
                    message: e.to_string(),
                });
            }
        }
    });
}

pub fn fetch_projects(
    state: &AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    if state.demo_mode {
        let space = state.current_space_name().to_string();
        let _ = tx.send(AppEvent::ProjectsLoaded {
            space,
            projects: mock::projects(),
        });
        return;
    }
    let space_name = state.current_space_name().to_string();
    let space_cfg = config
        .spaces
        .iter()
        .find(|s| s.name == space_name)
        .unwrap()
        .clone();
    tokio::spawn(async move {
        match api::client::BacklogClient::new(space_cfg.host, space_cfg.api_key) {
            Ok(client) => match client.fetch_projects().await {
                Ok(projects) => {
                    let _ = tx.send(AppEvent::ProjectsLoaded {
                        space: space_name,
                        projects,
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::ApiError {
                        space: space_name,
                        message: e.to_string(),
                    });
                }
            },
            Err(e) => {
                let _ = tx.send(AppEvent::ApiError {
                    space: space_name,
                    message: e.to_string(),
                });
            }
        }
    });
}
