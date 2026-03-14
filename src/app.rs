use std::collections::HashMap;

use crate::api::models::{Issue, IssueStatus, User};
use crate::config::Config;
use crate::event::AppEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    IssueList,
    IssueDetail,
    Filter,
}

#[derive(Debug, Clone, Default)]
pub struct SpaceState {
    pub issues: Option<Vec<Issue>>,
    pub users: Option<Vec<User>>,
    pub users_error: bool,
    pub loading_issues: bool,
}

pub struct AppState {
    pub config: Config,
    pub current_space_idx: usize,
    pub spaces: HashMap<String, SpaceState>,
    pub selected_issue_idx: usize,
    pub detail_issue: Option<Issue>,
    pub filter_assignee_id: Option<i64>,
    pub filter_cursor_idx: usize,
    pub screen: Screen,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let mut spaces = HashMap::new();
        for space in &config.spaces {
            spaces.insert(space.name.clone(), SpaceState::default());
        }
        let current_space_idx = config
            .spaces
            .iter()
            .position(|s| s.name == config.default_space)
            .unwrap_or(0);
        Self {
            config,
            current_space_idx,
            spaces,
            selected_issue_idx: 0,
            detail_issue: None,
            filter_assignee_id: None,
            filter_cursor_idx: 0,
            screen: Screen::IssueList,
            status_message: None,
            should_quit: false,
        }
    }

    pub fn current_space_name(&self) -> &str {
        &self.config.spaces[self.current_space_idx].name
    }

    pub fn current_space_state(&self) -> &SpaceState {
        self.spaces.get(self.current_space_name()).unwrap()
    }

    pub fn current_space_state_mut(&mut self) -> &mut SpaceState {
        let name = self.current_space_name().to_string();
        self.spaces.get_mut(&name).unwrap()
    }

    pub fn needs_issue_fetch(&self) -> bool {
        let state = self.current_space_state();
        state.issues.is_none() && !state.loading_issues
    }

    pub fn selected_issue(&self) -> Option<&Issue> {
        self.current_space_state()
            .issues
            .as_ref()
            .and_then(|issues| issues.get(self.selected_issue_idx))
    }

    pub fn navigate_down(&mut self) {
        let len = self
            .current_space_state()
            .issues
            .as_ref()
            .map(|v| v.len())
            .unwrap_or(0);
        if len > 0 && self.selected_issue_idx < len - 1 {
            self.selected_issue_idx += 1;
        }
    }

    pub fn navigate_up(&mut self) {
        if self.selected_issue_idx > 0 {
            self.selected_issue_idx -= 1;
        }
    }

    pub fn switch_space_next(&mut self) {
        self.current_space_idx = (self.current_space_idx + 1) % self.config.spaces.len();
        self.selected_issue_idx = 0;
        self.detail_issue = None;
        self.screen = Screen::IssueList;
    }

    pub fn switch_space_prev(&mut self) {
        if self.current_space_idx == 0 {
            self.current_space_idx = self.config.spaces.len() - 1;
        } else {
            self.current_space_idx -= 1;
        }
        self.selected_issue_idx = 0;
        self.detail_issue = None;
        self.screen = Screen::IssueList;
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::IssuesLoaded { space, issues } => {
                if let Some(state) = self.spaces.get_mut(&space) {
                    state.issues = Some(issues);
                    state.loading_issues = false;
                }
                self.selected_issue_idx = 0;
                self.status_message = None;
            }
            AppEvent::IssueDetailLoaded(issue) => {
                self.detail_issue = Some(issue);
                self.screen = Screen::IssueDetail;
            }
            AppEvent::SpaceUsersLoaded { space, users } => {
                if let Some(state) = self.spaces.get_mut(&space) {
                    state.users = Some(users);
                    state.users_error = false;
                }
            }
            AppEvent::ApiError { space, message } => {
                self.status_message = Some(format!("⚠ [{}] {}", space, message));
                if let Some(state) = self.spaces.get_mut(&space) {
                    state.loading_issues = false;
                    if state.users.is_none() {
                        state.users_error = true;
                    }
                }
            }
            AppEvent::Key(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(default: &str, names: &[&str]) -> Config {
        Config {
            default_space: default.to_string(),
            spaces: names
                .iter()
                .map(|n| crate::config::SpaceConfig {
                    name: n.to_string(),
                    host: format!("{}.backlog.com", n),
                    api_key: "key".to_string(),
                })
                .collect(),
        }
    }

    fn make_issue(key: &str) -> Issue {
        Issue {
            id: 1,
            issue_key: key.to_string(),
            summary: format!("Summary of {}", key),
            description: None,
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
    fn test_initial_state() {
        let config = make_config("space1", &["space1", "space2"]);
        let state = AppState::new(config);
        assert_eq!(state.current_space_name(), "space1");
        assert_eq!(state.current_space_idx, 0);
        assert_eq!(state.screen, Screen::IssueList);
        assert!(!state.should_quit);
    }

    #[test]
    fn test_default_space_selection() {
        let config = make_config("space2", &["space1", "space2"]);
        let state = AppState::new(config);
        assert_eq!(state.current_space_idx, 1);
        assert_eq!(state.current_space_name(), "space2");
    }

    #[test]
    fn test_issues_loaded_event() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![make_issue("PROJ-1"), make_issue("PROJ-2")],
        });
        let issues = state.current_space_state().issues.as_ref().unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].issue_key, "PROJ-1");
        assert!(!state.current_space_state().loading_issues);
        assert_eq!(state.selected_issue_idx, 0);
    }

    #[test]
    fn test_issue_detail_loaded_event() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::IssueDetailLoaded(make_issue("PROJ-5")));
        assert_eq!(state.screen, Screen::IssueDetail);
        assert_eq!(state.detail_issue.unwrap().issue_key, "PROJ-5");
    }

    #[test]
    fn test_space_users_loaded_event() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::SpaceUsersLoaded {
            space: "space1".to_string(),
            users: vec![
                User { id: 1, name: "Alice".to_string() },
                User { id: 2, name: "Bob".to_string() },
            ],
        });
        let users = state.current_space_state().users.as_ref().unwrap();
        assert_eq!(users.len(), 2);
        assert!(!state.current_space_state().users_error);
    }

    #[test]
    fn test_api_error_sets_status_message() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::ApiError {
            space: "space1".to_string(),
            message: "401 Unauthorized".to_string(),
        });
        let msg = state.status_message.unwrap();
        assert!(msg.contains("space1"));
        assert!(msg.contains("401 Unauthorized"));
    }

    #[test]
    fn test_api_error_sets_users_error_when_no_users() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::ApiError {
            space: "space1".to_string(),
            message: "timeout".to_string(),
        });
        assert!(state.current_space_state().users_error);
    }

    #[test]
    fn test_navigate_down() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![make_issue("PROJ-1"), make_issue("PROJ-2"), make_issue("PROJ-3")],
        });
        state.navigate_down();
        assert_eq!(state.selected_issue_idx, 1);
        state.navigate_down();
        assert_eq!(state.selected_issue_idx, 2);
        state.navigate_down(); // at end, should not go past
        assert_eq!(state.selected_issue_idx, 2);
    }

    #[test]
    fn test_navigate_up() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![make_issue("PROJ-1"), make_issue("PROJ-2")],
        });
        state.navigate_down();
        state.navigate_up();
        assert_eq!(state.selected_issue_idx, 0);
        state.navigate_up(); // at top, should not go negative
        assert_eq!(state.selected_issue_idx, 0);
    }

    #[test]
    fn test_switch_space_next() {
        let config = make_config("space1", &["space1", "space2", "space3"]);
        let mut state = AppState::new(config);
        state.switch_space_next();
        assert_eq!(state.current_space_name(), "space2");
        state.switch_space_next();
        assert_eq!(state.current_space_name(), "space3");
        state.switch_space_next(); // wraps around
        assert_eq!(state.current_space_name(), "space1");
    }

    #[test]
    fn test_switch_space_prev() {
        let config = make_config("space1", &["space1", "space2", "space3"]);
        let mut state = AppState::new(config);
        state.switch_space_prev(); // wraps around
        assert_eq!(state.current_space_name(), "space3");
        state.switch_space_prev();
        assert_eq!(state.current_space_name(), "space2");
    }

    #[test]
    fn test_needs_issue_fetch_true_when_no_issues() {
        let config = make_config("space1", &["space1"]);
        let state = AppState::new(config);
        assert!(state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_false_when_loading() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.current_space_state_mut().loading_issues = true;
        assert!(!state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_false_when_loaded() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![],
        });
        assert!(!state.needs_issue_fetch());
    }
}
