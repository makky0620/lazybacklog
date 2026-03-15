use std::collections::HashMap;

use crate::api::models::{Issue, IssueStatus, Project, User};
use crate::config::Config;
use crate::event::AppEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    ProjectSelect,
    IssueList,
    IssueDetail,
    Filter,
    StatusFilter,
}

#[derive(Debug, Clone, Default)]
pub struct SpaceState {
    pub issues: Option<Vec<Issue>>,
    pub users: Option<Vec<User>>,
    pub users_error: bool,
    pub loading_issues: bool,
    pub projects: Option<Vec<Project>>,
    pub loading_projects: bool,
    pub selected_project: Option<Project>,
    pub statuses: Option<Vec<IssueStatus>>,
    pub loading_statuses: bool,
    pub filter_status_ids: Vec<i64>,
}

pub struct AppState {
    pub config: Config,
    pub current_space_idx: usize,
    pub spaces: HashMap<String, SpaceState>,
    pub selected_issue_idx: usize,
    pub detail_issue: Option<Issue>,
    pub filter_assignee_id: Option<i64>,
    pub filter_cursor_idx: usize,
    pub project_cursor_idx: usize,
    pub screen: Screen,
    pub status_message: Option<String>,
    pub should_quit: bool,
    pub status_filter_cursor_idx: usize,
    pub status_filter_pending: Vec<i64>,
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
            project_cursor_idx: 0,
            screen: Screen::ProjectSelect,
            status_message: None,
            should_quit: false,
            status_filter_cursor_idx: 0,
            status_filter_pending: vec![],
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
        state.statuses.is_some()
            && state.issues.is_none()
            && !state.loading_issues
            && !state.loading_statuses
    }

    pub fn needs_projects_fetch(&self) -> bool {
        let state = self.current_space_state();
        state.projects.is_none() && !state.loading_projects
    }

    pub fn selected_project(&self) -> Option<&Project> {
        self.current_space_state().selected_project.as_ref()
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
        self.project_cursor_idx = 0;
        self.filter_assignee_id = None;
        self.screen = Screen::ProjectSelect;
    }

    pub fn switch_space_prev(&mut self) {
        if self.current_space_idx == 0 {
            self.current_space_idx = self.config.spaces.len() - 1;
        } else {
            self.current_space_idx -= 1;
        }
        self.selected_issue_idx = 0;
        self.detail_issue = None;
        self.project_cursor_idx = 0;
        self.filter_assignee_id = None;
        self.screen = Screen::ProjectSelect;
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
            AppEvent::ProjectsLoaded { space, projects } => {
                if let Some(state) = self.spaces.get_mut(&space) {
                    state.projects = Some(projects);
                    state.loading_projects = false;
                }
            }
            AppEvent::ApiError { space, message } => {
                self.status_message = Some(format!("⚠ [{}] {}", space, message));
                if let Some(state) = self.spaces.get_mut(&space) {
                    state.loading_issues = false;
                    state.loading_projects = false;
                    state.loading_statuses = false;
                    if state.statuses.is_none() {
                        state.statuses = Some(vec![]);
                    }
                    if state.users.is_none() {
                        state.users_error = true;
                    }
                }
            }
            AppEvent::StatusesLoaded { space, statuses } => {
                if let Some(state) = self.spaces.get_mut(&space) {
                    let default_ids: Vec<i64> = statuses
                        .iter()
                        .filter(|s| s.name != "完了" && s.name != "Closed")
                        .map(|s| s.id)
                        .collect();
                    state.filter_status_ids = default_ids;
                    state.statuses = Some(statuses);
                    state.loading_statuses = false;
                }
            }
            AppEvent::Key(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::IssueStatus;

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
        assert_eq!(state.screen, Screen::ProjectSelect);
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
                User {
                    id: 1,
                    name: "Alice".to_string(),
                },
                User {
                    id: 2,
                    name: "Bob".to_string(),
                },
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
            issues: vec![
                make_issue("PROJ-1"),
                make_issue("PROJ-2"),
                make_issue("PROJ-3"),
            ],
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
    fn test_needs_issue_fetch_false_when_statuses_not_loaded() {
        let config = make_config("space1", &["space1"]);
        let state = AppState::new(config);
        assert!(!state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_true_when_statuses_loaded() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.current_space_state_mut().statuses = Some(vec![]);
        assert!(state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_true_when_no_issues() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.current_space_state_mut().statuses = Some(vec![]);
        assert!(state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_false_when_loading() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.current_space_state_mut().statuses = Some(vec![]);
        state.current_space_state_mut().loading_issues = true;
        assert!(!state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_false_when_loaded() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.current_space_state_mut().statuses = Some(vec![]);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![],
        });
        assert!(!state.needs_issue_fetch());
    }

    #[test]
    fn test_projects_loaded_event() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::ProjectsLoaded {
            space: "space1".to_string(),
            projects: vec![crate::api::models::Project {
                id: 1,
                project_key: "PROJ".to_string(),
                name: "My Project".to_string(),
            }],
        });
        let projects = state.current_space_state().projects.as_ref().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].project_key, "PROJ");
        assert!(!state.current_space_state().loading_projects);
    }

    #[test]
    fn test_api_error_resets_loading_projects() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.current_space_state_mut().loading_projects = true;
        state.handle_event(AppEvent::ApiError {
            space: "space1".to_string(),
            message: "timeout".to_string(),
        });
        assert!(!state.current_space_state().loading_projects);
    }

    #[test]
    fn test_needs_projects_fetch_true_when_no_projects() {
        let config = make_config("space1", &["space1"]);
        let state = AppState::new(config);
        assert!(state.needs_projects_fetch());
    }

    #[test]
    fn test_needs_projects_fetch_false_when_loading() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.current_space_state_mut().loading_projects = true;
        assert!(!state.needs_projects_fetch());
    }

    #[test]
    fn test_needs_projects_fetch_false_when_loaded() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::ProjectsLoaded {
            space: "space1".to_string(),
            projects: vec![],
        });
        assert!(!state.needs_projects_fetch());
    }

    #[test]
    fn test_switch_space_resets_project_state() {
        let config = make_config("space1", &["space1", "space2"]);
        let mut state = AppState::new(config);
        state.filter_assignee_id = Some(42);
        state.project_cursor_idx = 3;
        state.switch_space_next();
        assert_eq!(state.screen, Screen::ProjectSelect);
        assert_eq!(state.project_cursor_idx, 0);
        assert!(state.filter_assignee_id.is_none());
    }

    fn make_status(id: i64, name: &str) -> IssueStatus {
        IssueStatus {
            id,
            name: name.to_string(),
        }
    }

    #[test]
    fn test_statuses_loaded_sets_default_filter_excluding_closed() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::StatusesLoaded {
            space: "space1".to_string(),
            statuses: vec![
                make_status(1, "未対応"),
                make_status(2, "処理中"),
                make_status(3, "処理済み"),
                make_status(4, "完了"),
            ],
        });
        let ss = state.current_space_state();
        assert!(ss.statuses.is_some());
        assert!(!ss.loading_statuses);
        assert_eq!(ss.filter_status_ids, vec![1, 2, 3]);
    }

    #[test]
    fn test_statuses_loaded_excludes_closed_english() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::StatusesLoaded {
            space: "space1".to_string(),
            statuses: vec![
                make_status(1, "Open"),
                make_status(2, "In Progress"),
                make_status(3, "Resolved"),
                make_status(4, "Closed"),
            ],
        });
        let ss = state.current_space_state();
        assert_eq!(ss.filter_status_ids, vec![1, 2, 3]);
    }

    #[test]
    fn test_statuses_loaded_all_open_no_exclusion() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::StatusesLoaded {
            space: "space1".to_string(),
            statuses: vec![make_status(1, "In Progress"), make_status(2, "Review")],
        });
        let ss = state.current_space_state();
        assert_eq!(ss.filter_status_ids, vec![1, 2]);
    }

    #[test]
    fn test_statuses_loaded_wrong_space_is_noop() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::StatusesLoaded {
            space: "nonexistent".to_string(),
            statuses: vec![make_status(1, "Open")],
        });
        assert!(state.current_space_state().statuses.is_none());
        assert!(state.current_space_state().filter_status_ids.is_empty());
    }

    #[test]
    fn test_api_error_resets_loading_statuses() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.current_space_state_mut().loading_statuses = true;
        state.handle_event(AppEvent::ApiError {
            space: "space1".to_string(),
            message: "timeout".to_string(),
        });
        assert!(!state.current_space_state().loading_statuses);
        assert!(state.current_space_state().statuses.is_some());
    }

    #[test]
    fn test_space_state_default_statuses_is_none() {
        let config = make_config("space1", &["space1"]);
        let state = AppState::new(config);
        assert!(state.current_space_state().statuses.is_none());
        assert!(!state.current_space_state().loading_statuses);
        assert!(state.current_space_state().filter_status_ids.is_empty());
    }

    #[test]
    fn test_appstate_default_status_filter_fields() {
        let config = make_config("space1", &["space1"]);
        let state = AppState::new(config);
        assert_eq!(state.status_filter_cursor_idx, 0);
        assert!(state.status_filter_pending.is_empty());
    }
}
