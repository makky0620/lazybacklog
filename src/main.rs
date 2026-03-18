use anyhow::Result;
// NOTE: Do NOT import `crossterm::event` with `{self, ...}` — the local `mod event`
// module creates an E0255 name conflict. Use only the specific types we need.
use crossterm::{
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
mod handler;
mod mock;
mod ui;

use app::{AppState, Screen};
use event::AppEvent;

#[tokio::main]
async fn main() -> Result<()> {
    let demo_mode = std::env::args().any(|a| a == "--demo");

    let config = if demo_mode {
        mock::demo_config()
    } else {
        config::load().unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        })
    };

    #[cfg(unix)]
    if !demo_mode {
        if let Some(warning) = config::check_permissions(&config::config_path()) {
            eprintln!("{}", warning);
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, config, demo_mode).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    config: config::Config,
    demo_mode: bool,
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

    let mut state = AppState::new(config.clone(), demo_mode);

    // Set loading_projects = true for ALL spaces before spawning, to prevent
    // needs_projects_fetch() from firing while startup tasks are in flight.
    for space in &config.spaces {
        state.spaces.get_mut(&space.name).unwrap().loading_projects = true;
    }

    // Spawn per-space tasks: fetch projects (for ProjectsLoaded) and users.
    if demo_mode {
        for space in &config.spaces {
            let _ = tx.send(AppEvent::ProjectsLoaded {
                space: space.name.clone(),
                projects: mock::projects(),
            });
            let _ = tx.send(AppEvent::SpaceUsersLoaded {
                space: space.name.clone(),
                users: mock::users(),
            });
        }
    } else {
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
    }
    // No initial fetch_issues — user selects project first.

    loop {
        terminal.draw(|f| ui::render(f, &state))?;

        if let Some(ev) = rx.recv().await {
            match ev {
                AppEvent::Key(key) => match state.screen {
                    Screen::SpaceSelect => {
                        handler::handle_space_select_key(key, &mut state, &config)
                    }
                    Screen::IssueList => {
                        handler::handle_list_key(key, &mut state, &config, tx.clone())
                    }
                    Screen::IssueDetail => handler::handle_detail_key(key, &mut state),
                    Screen::Filter => {
                        handler::handle_filter_key(key, &mut state, &config, tx.clone())
                    }
                    Screen::ProjectSelect => {
                        handler::handle_project_select_key(key, &mut state, &config, tx.clone())
                    }
                    Screen::StatusFilter => {
                        handler::handle_status_filter_key(key, &mut state, &config, tx.clone())
                    }
                },
                other => {
                    state.handle_event(other);
                    // Guard 1: issue auto-fetch (only on IssueList screen)
                    if state.screen == Screen::IssueList && state.needs_issue_fetch() {
                        let project_id = state.selected_project().map(|p| p.id);
                        let assignee_id = state.filter_assignee_id;
                        let status_ids = state.current_space_state().filter_status_ids.clone();
                        handler::fetch_issues(
                            &state,
                            &config,
                            tx.clone(),
                            project_id,
                            assignee_id,
                            status_ids,
                        );
                        state.current_space_state_mut().loading_issues = true;
                    }
                    // Guard 2: project auto-fetch (only on ProjectSelect screen)
                    // Fires when user switches to a space whose projects were not yet loaded.
                    if state.screen == Screen::ProjectSelect && state.needs_projects_fetch() {
                        handler::fetch_projects(&state, &config, tx.clone());
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
