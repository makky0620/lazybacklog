use anyhow::Result;
// NOTE: Do NOT import `crossterm::event` with `{self, ...}` — the local `mod event`
// module creates an E0255 name conflict. Use only the specific types we need.
use crossterm::{
    event::KeyCode,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc;

mod api;
mod app;
mod config;
mod event;
mod ui;

use app::{AppState, Screen};
use event::AppEvent;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::load().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    #[cfg(unix)]
    if let Some(warning) = config::check_permissions(&config::config_path()) {
        eprintln!("{}", warning);
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, config).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    config: config::Config,
) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // Spawn key-reading thread — sends Key events to the shared channel.
    // IMPORTANT: Use fully-qualified crossterm::event::read() and
    // crossterm::event::Event::Key — NOT event::read() (which would look up
    // src/event.rs, which has no read() function).
    let key_tx = tx.clone();
    std::thread::spawn(move || loop {
        if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
            if key_tx.send(AppEvent::Key(key)).is_err() {
                break;
            }
        }
    });

    let mut state = AppState::new(config.clone());

    // Set loading_projects = true for ALL spaces before spawning, to prevent
    // needs_projects_fetch() from firing while startup tasks are in flight.
    for space in &config.spaces {
        state.spaces.get_mut(&space.name).unwrap().loading_projects = true;
    }

    // Spawn per-space tasks: fetch projects (for ProjectsLoaded) and users.
    for space in &config.spaces {
        let space_name = space.name.clone();
        let host = space.host.clone();
        let api_key = space.api_key.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match api::client::BacklogClient::new(host, api_key) {
                Ok(client) => match client.fetch_projects().await {
                    Ok(projects) => {
                        // Send ProjectsLoaded for every space.
                        // Clone into the event; borrow original for user iteration below.
                        let _ = tx.send(AppEvent::ProjectsLoaded {
                            space: space_name.clone(),
                            projects: projects.clone(),
                        });
                        // Fetch users for each project (iterate by reference, not by move).
                        let mut all_users: Vec<api::models::User> = Vec::new();
                        for project in &projects {
                            if let Ok(users) = client.fetch_project_users(project.id).await {
                                for user in users {
                                    if !all_users.iter().any(|u| u.id == user.id) {
                                        all_users.push(user);
                                    }
                                }
                            }
                        }
                        let _ = tx.send(AppEvent::SpaceUsersLoaded {
                            space: space_name,
                            users: all_users,
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
    // No initial fetch_issues — user selects project first.

    loop {
        terminal.draw(|f| ui::render(f, &state))?;

        if let Some(ev) = rx.recv().await {
            match ev {
                AppEvent::Key(key) => match state.screen {
                    Screen::IssueList => handle_list_key(key, &mut state, &config, tx.clone()),
                    Screen::IssueDetail => handle_detail_key(key, &mut state),
                    Screen::Filter => handle_filter_key(key, &mut state, &config, tx.clone()),
                    Screen::ProjectSelect => handle_project_select_key(key, &mut state, &config, tx.clone()),
                    Screen::StatusFilter => handle_status_filter_key(key, &mut state, &config, tx.clone()),
                },
                other => {
                    state.handle_event(other);
                    // Guard 1: issue auto-fetch (only on IssueList screen)
                    if state.screen == Screen::IssueList && state.needs_issue_fetch() {
                        let project_id = state.selected_project().map(|p| p.id);
                        let assignee_id = state.filter_assignee_id;
                        let status_ids = state.current_space_state().filter_status_ids.clone();
                        fetch_issues(&state, &config, tx.clone(), project_id, assignee_id, status_ids);
                        state.current_space_state_mut().loading_issues = true;
                    }
                    // Guard 2: project auto-fetch (only on ProjectSelect screen)
                    // Fires when user switches to a space whose projects were not yet loaded.
                    if state.screen == Screen::ProjectSelect && state.needs_projects_fetch() {
                        fetch_projects(&state, &config, tx.clone());
                        state.current_space_state_mut().loading_projects = true;
                    }
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_list_key(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    match key.code {
        KeyCode::Char('q') => state.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => state.navigate_down(),
        KeyCode::Char('k') | KeyCode::Up => state.navigate_up(),
        KeyCode::Enter => {
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
        KeyCode::Char('r') => {
            let project_id = state.selected_project().map(|p| p.id);
            let assignee_id = state.filter_assignee_id;
            let status_ids = state.current_space_state().filter_status_ids.clone();
            state.current_space_state_mut().issues = None;
            state.current_space_state_mut().loading_issues = true;
            fetch_issues(state, config, tx, project_id, assignee_id, status_ids);
        }
        KeyCode::Char(']') => {
            state.switch_space_next();
        }
        KeyCode::Char('[') => {
            state.switch_space_prev();
        }
        _ => {}
    }
}

fn handle_detail_key(key: crossterm::event::KeyEvent, state: &mut AppState) {
    if key.code == KeyCode::Esc {
        state.screen = Screen::IssueList;
        state.detail_issue = None;
    }
}

fn handle_filter_key(
    key: crossterm::event::KeyEvent,
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

    match key.code {
        KeyCode::Esc => state.screen = Screen::IssueList,
        KeyCode::Char('j') | KeyCode::Down => {
            if state.filter_cursor_idx + 1 < total {
                state.filter_cursor_idx += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.filter_cursor_idx > 0 {
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
            state.screen = Screen::IssueList;
            let project_id = state.selected_project().map(|p| p.id);
            let assignee_id = state.filter_assignee_id;
            let status_ids = state.current_space_state().filter_status_ids.clone();
            state.current_space_state_mut().issues = None;
            state.current_space_state_mut().loading_issues = true;
            fetch_issues(state, config, tx, project_id, assignee_id, status_ids);
        }
        _ => {}
    }
}

fn handle_status_filter_key(
    key: crossterm::event::KeyEvent,
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

    match key.code {
        KeyCode::Esc => {
            state.status_filter_pending = vec![];
            state.screen = Screen::IssueList;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if status_count > 0 && state.status_filter_cursor_idx + 1 < status_count {
                state.status_filter_cursor_idx += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.status_filter_cursor_idx > 0 {
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
            state.screen = Screen::IssueList;
            let project_id = state.selected_project().map(|p| p.id);
            let assignee_id = state.filter_assignee_id;
            let status_ids = state.current_space_state().filter_status_ids.clone();
            state.current_space_state_mut().issues = None;
            state.current_space_state_mut().loading_issues = true;
            fetch_issues(state, config, tx, project_id, assignee_id, status_ids);
        }
        _ => {}
    }
}

fn handle_project_select_key(
    key: crossterm::event::KeyEvent,
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

fn fetch_issues(
    state: &AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
    project_id: Option<i64>,
    assignee_id: Option<i64>,
    status_ids: Vec<i64>,
) {
    let space_name = state.current_space_name().to_string();
    let space_cfg = config
        .spaces
        .iter()
        .find(|s| s.name == space_name)
        .unwrap()
        .clone();
    tokio::spawn(async move {
        match api::client::BacklogClient::new(space_cfg.host, space_cfg.api_key) {
            Ok(client) => match client.fetch_issues(project_id, assignee_id, &status_ids).await {
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

fn fetch_statuses(
    state: &AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
    project_id: i64,
) {
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

fn fetch_projects(
    state: &AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
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
