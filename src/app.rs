use std::collections::HashMap;

use crate::api::models::{Comment, Issue, IssueStatus, Project, User};
use crate::config::Config;
use crate::event::AppEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    SpaceSelect, // NEW — initial screen
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
    pub demo_mode: bool,
    pub config: Config,
    pub current_space_idx: usize,
    pub space_cursor_idx: usize,
    pub spaces: HashMap<String, SpaceState>,
    pub selected_issue_idx: usize,
    pub detail_issue: Option<Issue>,
    pub detail_comments: Option<Vec<Comment>>,
    pub filter_assignee_id: Option<i64>,
    pub filter_cursor_idx: usize,
    pub project_cursor_idx: usize,
    pub screen: Screen,
    pub status_message: Option<String>,
    pub should_quit: bool,
    pub detail_scroll_offset: u16,
    pub status_filter_cursor_idx: usize,
    pub status_filter_pending: Vec<i64>,
    pub search_active: bool,
    pub search_query: String,
    pub search_match_idx: usize,
}

impl AppState {
    pub fn new(config: Config, demo_mode: bool) -> Self {
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
            demo_mode,
            config,
            current_space_idx,
            space_cursor_idx: current_space_idx,
            spaces,
            selected_issue_idx: 0,
            detail_issue: None,
            detail_comments: None,
            filter_assignee_id: None,
            filter_cursor_idx: 0,
            project_cursor_idx: 0,
            screen: Screen::SpaceSelect,
            status_message: None,
            should_quit: false,
            detail_scroll_offset: 0,
            status_filter_cursor_idx: 0,
            status_filter_pending: vec![],
            search_active: false,
            search_query: String::new(),
            search_match_idx: 0,
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
        if len == 0 {
            return;
        }
        if self.search_query.is_empty() {
            if self.selected_issue_idx < len - 1 {
                self.selected_issue_idx += 1;
            }
        } else {
            let matches = self.matching_issue_indices();
            if let Some(pos) = matches.iter().position(|&i| i > self.selected_issue_idx) {
                self.selected_issue_idx = matches[pos];
                self.search_match_idx = pos;
            }
        }
    }

    pub fn navigate_up(&mut self) {
        if self.search_query.is_empty() {
            if self.selected_issue_idx > 0 {
                self.selected_issue_idx -= 1;
            }
        } else {
            let matches = self.matching_issue_indices();
            if let Some(pos) = matches.iter().rposition(|&i| i < self.selected_issue_idx) {
                self.selected_issue_idx = matches[pos];
                self.search_match_idx = pos;
            }
        }
    }

    pub fn select_space(&mut self, idx: usize) {
        self.clear_search();
        self.selected_issue_idx = 0;
        self.detail_issue = None;
        self.detail_comments = None;
        self.detail_scroll_offset = 0;
        self.project_cursor_idx = 0;
        self.filter_assignee_id = None;
        self.current_space_idx = idx;
        self.screen = Screen::ProjectSelect;
    }

    pub fn clear_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_match_idx = 0;
        self.filter_cursor_idx = 0;
        self.status_filter_cursor_idx = 0;
    }

    /// Returns full-list indices of issues matching the current search_query.
    /// Returns all indices when query is empty.
    pub fn matching_issue_indices(&self) -> Vec<usize> {
        let Some(issues) = self.current_space_state().issues.as_ref() else {
            return vec![];
        };
        if self.search_query.is_empty() {
            return (0..issues.len()).collect();
        }
        let query = self.search_query.to_lowercase();
        issues
            .iter()
            .enumerate()
            .filter(|(_, issue)| {
                issue.issue_key.to_lowercase().contains(&query)
                    || issue.summary.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Returns full-list indices of users (and ALL row at 0) matching the current search_query.
    /// Index 0 = "ALL" row, indices 1.. = users.
    /// Returns all indices when query is empty.
    pub fn matching_user_indices(&self) -> Vec<usize> {
        let space_state = self.current_space_state();
        let user_count = space_state.users.as_ref().map(|u| u.len()).unwrap_or(0);
        if self.search_query.is_empty() {
            return (0..=user_count).collect();
        }
        let query = self.search_query.to_lowercase();
        let mut indices = vec![];
        if "all".contains(&query) {
            indices.push(0);
        }
        if let Some(users) = &space_state.users {
            for (i, user) in users.iter().enumerate() {
                if user.name.to_lowercase().contains(&query) {
                    indices.push(i + 1);
                }
            }
        }
        indices
    }

    /// Returns full-list indices of statuses matching the current search_query.
    /// Returns all indices when query is empty.
    pub fn matching_status_indices(&self) -> Vec<usize> {
        let Some(statuses) = self.current_space_state().statuses.as_ref() else {
            return vec![];
        };
        if self.search_query.is_empty() {
            return (0..statuses.len()).collect();
        }
        let query = self.search_query.to_lowercase();
        statuses
            .iter()
            .enumerate()
            .filter(|(_, s)| s.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect()
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
                self.clear_search();
                self.detail_issue = Some(issue);
                self.detail_scroll_offset = 0;
                self.detail_comments = None;
                self.screen = Screen::IssueDetail;
            }
            AppEvent::CommentsLoaded {
                issue_key,
                comments,
            } => {
                if self.detail_issue.as_ref().map(|i| &i.issue_key) == Some(&issue_key) {
                    self.detail_comments = Some(comments);
                }
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
        let state = AppState::new(config, false);
        assert_eq!(state.current_space_name(), "space1");
        assert_eq!(state.current_space_idx, 0);
        assert_eq!(state.screen, Screen::SpaceSelect);
        assert_eq!(state.space_cursor_idx, state.current_space_idx);
        assert!(!state.should_quit);
    }

    #[test]
    fn test_default_space_selection() {
        let config = make_config("space2", &["space1", "space2"]);
        let state = AppState::new(config, false);
        assert_eq!(state.current_space_idx, 1);
        assert_eq!(state.current_space_name(), "space2");
        assert_eq!(state.space_cursor_idx, state.current_space_idx);
    }

    #[test]
    fn test_issues_loaded_event() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
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
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::IssueDetailLoaded(make_issue("PROJ-5")));
        assert_eq!(state.screen, Screen::IssueDetail);
        assert_eq!(state.detail_issue.unwrap().issue_key, "PROJ-5");
    }

    #[test]
    fn test_space_users_loaded_event() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
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
        let mut state = AppState::new(config, false);
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
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::ApiError {
            space: "space1".to_string(),
            message: "timeout".to_string(),
        });
        assert!(state.current_space_state().users_error);
    }

    #[test]
    fn test_navigate_down() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
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
        let mut state = AppState::new(config, false);
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
    fn test_needs_issue_fetch_false_when_statuses_not_loaded() {
        let config = make_config("space1", &["space1"]);
        let state = AppState::new(config, false);
        assert!(!state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_true_when_statuses_loaded() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.current_space_state_mut().statuses = Some(vec![]);
        assert!(state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_true_when_no_issues() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.current_space_state_mut().statuses = Some(vec![]);
        assert!(state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_false_when_loading() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.current_space_state_mut().statuses = Some(vec![]);
        state.current_space_state_mut().loading_issues = true;
        assert!(!state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_false_when_loaded() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
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
        let mut state = AppState::new(config, false);
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
        let mut state = AppState::new(config, false);
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
        let state = AppState::new(config, false);
        assert!(state.needs_projects_fetch());
    }

    #[test]
    fn test_needs_projects_fetch_false_when_loading() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.current_space_state_mut().loading_projects = true;
        assert!(!state.needs_projects_fetch());
    }

    #[test]
    fn test_needs_projects_fetch_false_when_loaded() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::ProjectsLoaded {
            space: "space1".to_string(),
            projects: vec![],
        });
        assert!(!state.needs_projects_fetch());
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
        let mut state = AppState::new(config, false);
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
        let mut state = AppState::new(config, false);
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
        let mut state = AppState::new(config, false);
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
        let mut state = AppState::new(config, false);
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
        let mut state = AppState::new(config, false);
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
        let state = AppState::new(config, false);
        assert!(state.current_space_state().statuses.is_none());
        assert!(!state.current_space_state().loading_statuses);
        assert!(state.current_space_state().filter_status_ids.is_empty());
    }

    #[test]
    fn test_appstate_default_status_filter_fields() {
        let config = make_config("space1", &["space1"]);
        let state = AppState::new(config, false);
        assert_eq!(state.status_filter_cursor_idx, 0);
        assert!(state.status_filter_pending.is_empty());
    }

    #[test]
    fn test_clear_search_resets_all() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.search_active = true;
        state.search_query = "foo".to_string();
        state.search_match_idx = 2;
        state.selected_issue_idx = 5;
        state.clear_search();
        assert!(!state.search_active);
        assert!(state.search_query.is_empty());
        assert_eq!(state.search_match_idx, 0);
        assert_eq!(state.selected_issue_idx, 5); // selected_issue_idx is NOT reset by clear_search
        assert_eq!(state.filter_cursor_idx, 0);
        assert_eq!(state.status_filter_cursor_idx, 0);
    }

    #[test]
    fn test_matching_issue_indices_empty_query_returns_all() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![
                make_issue("PROJ-1"),
                make_issue("PROJ-2"),
                make_issue("PROJ-3"),
            ],
        });
        let indices = state.matching_issue_indices();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_matching_issue_indices_filters_by_key() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![
                make_issue("PROJ-1"),
                make_issue("PROJ-2"),
                make_issue("ABC-1"),
            ],
        });
        state.search_query = "proj".to_string();
        let indices = state.matching_issue_indices();
        assert_eq!(indices, vec![0, 1]);
    }

    #[test]
    fn test_matching_issue_indices_filters_by_summary() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        // make_issue() sets summary = "Summary of <key>"
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![make_issue("PROJ-1"), make_issue("ABC-1")],
        });
        state.search_query = "summary of proj".to_string();
        let indices = state.matching_issue_indices();
        assert_eq!(indices, vec![0]);
    }

    #[test]
    fn test_matching_issue_indices_case_insensitive() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![make_issue("PROJ-1")],
        });
        state.search_query = "PROJ".to_string();
        let indices = state.matching_issue_indices();
        assert_eq!(indices, vec![0]);
    }

    #[test]
    fn test_matching_issue_indices_no_match_returns_empty() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![make_issue("PROJ-1")],
        });
        state.search_query = "zzz".to_string();
        let indices = state.matching_issue_indices();
        assert!(indices.is_empty());
    }

    #[test]
    fn test_matching_issue_indices_no_issues_returns_empty() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.search_query = "proj".to_string();
        // No IssuesLoaded event — issues is None
        let indices = state.matching_issue_indices();
        assert!(indices.is_empty());
    }

    #[test]
    fn test_matching_user_indices_empty_query_returns_all() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
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
        // 0 = ALL, 1 = Alice, 2 = Bob
        let indices = state.matching_user_indices();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_matching_user_indices_query_all_matches_all_row() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::SpaceUsersLoaded {
            space: "space1".to_string(),
            users: vec![User {
                id: 1,
                name: "Alice".to_string(),
            }],
        });
        state.search_query = "all".to_string();
        let indices = state.matching_user_indices();
        assert!(indices.contains(&0)); // "ALL" row matches
    }

    #[test]
    fn test_matching_user_indices_filters_by_name() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
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
        state.search_query = "alice".to_string();
        let indices = state.matching_user_indices();
        assert_eq!(indices, vec![1]); // index 1 = Alice, ALL row doesn't match "alice"
    }

    #[test]
    fn test_matching_status_indices_empty_query_returns_all() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::StatusesLoaded {
            space: "space1".to_string(),
            statuses: vec![
                make_status(1, "Open"),
                make_status(2, "In Progress"),
                make_status(3, "Closed"),
            ],
        });
        let indices = state.matching_status_indices();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_matching_status_indices_filters_by_name() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::StatusesLoaded {
            space: "space1".to_string(),
            statuses: vec![
                make_status(1, "Open"),
                make_status(2, "In Progress"),
                make_status(3, "Closed"),
            ],
        });
        state.search_query = "open".to_string();
        let indices = state.matching_status_indices();
        assert_eq!(indices, vec![0]);
    }

    #[test]
    fn test_matching_status_indices_no_statuses_returns_empty() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.search_query = "open".to_string();
        let indices = state.matching_status_indices();
        assert!(indices.is_empty());
    }

    #[test]
    fn test_navigate_down_with_search_skips_non_matching() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![
                make_issue("PROJ-1"),
                make_issue("ABC-1"), // non-matching
                make_issue("PROJ-2"),
            ],
        });
        state.search_query = "proj".to_string();
        state.selected_issue_idx = 0; // at PROJ-1
        state.navigate_down();
        assert_eq!(state.selected_issue_idx, 2); // jumps to PROJ-2, skips ABC-1
    }

    #[test]
    fn test_navigate_down_with_search_no_further_match_stays() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![make_issue("PROJ-1"), make_issue("ABC-1")],
        });
        state.search_query = "proj".to_string();
        state.selected_issue_idx = 0; // at PROJ-1, last match
        state.navigate_down();
        assert_eq!(state.selected_issue_idx, 0); // stays
    }

    #[test]
    fn test_navigate_up_with_search_skips_non_matching() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![
                make_issue("PROJ-1"),
                make_issue("ABC-1"), // non-matching
                make_issue("PROJ-2"),
            ],
        });
        state.search_query = "proj".to_string();
        state.selected_issue_idx = 2; // at PROJ-2
        state.navigate_up();
        assert_eq!(state.selected_issue_idx, 0); // jumps to PROJ-1, skips ABC-1
    }

    #[test]
    fn test_navigate_up_with_search_no_earlier_match_stays() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![make_issue("ABC-1"), make_issue("PROJ-1")],
        });
        state.search_query = "proj".to_string();
        state.selected_issue_idx = 1; // at PROJ-1, first match
        state.navigate_up();
        assert_eq!(state.selected_issue_idx, 1); // stays
    }

    #[test]
    fn test_issue_detail_loaded_clears_search() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.search_active = true;
        state.search_query = "proj".to_string();
        state.handle_event(AppEvent::IssueDetailLoaded(make_issue("PROJ-1")));
        assert!(!state.search_active);
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn test_issue_detail_loaded_preserves_cursor_position() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![
                make_issue("PROJ-1"),
                make_issue("PROJ-2"),
                make_issue("PROJ-3"),
            ],
        });
        state.selected_issue_idx = 2;
        state.search_active = true;
        state.search_query = "proj".to_string();
        state.handle_event(AppEvent::IssueDetailLoaded(make_issue("PROJ-3")));
        assert_eq!(state.selected_issue_idx, 2); // cursor preserved
        assert!(!state.search_active); // search still cleared
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn test_detail_scroll_offset_initial_zero() {
        let config = make_config("space1", &["space1"]);
        let state = AppState::new(config, false);
        assert_eq!(state.detail_scroll_offset, 0);
    }

    #[test]
    fn test_detail_scroll_offset_reset_on_issue_detail_loaded() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.detail_scroll_offset = 5;
        state.handle_event(AppEvent::IssueDetailLoaded(make_issue("PROJ-1")));
        assert_eq!(state.detail_scroll_offset, 0);
    }

    #[test]
    fn test_select_space_resets_detail_scroll_offset() {
        let config = make_config("space1", &["space1", "space2"]);
        let mut state = AppState::new(config, false);
        state.detail_scroll_offset = 7;
        state.select_space(1);
        assert_eq!(state.detail_scroll_offset, 0);
    }

    #[test]
    fn test_space_cursor_idx_initial_value() {
        let config = make_config("space2", &["space1", "space2"]);
        let state = AppState::new(config, false);
        assert_eq!(state.current_space_idx, 1);
        assert_eq!(state.space_cursor_idx, 1);
    }

    #[test]
    fn test_select_space_resets_state() {
        let config = make_config("space1", &["space1", "space2"]);
        let mut state = AppState::new(config, false);
        state.space_cursor_idx = 1;
        state.filter_assignee_id = Some(42);
        state.project_cursor_idx = 3;
        state.selected_issue_idx = 5;
        state.search_active = true;
        state.search_query = "proj".to_string();

        let idx = state.space_cursor_idx;
        state.select_space(idx);

        assert_eq!(state.current_space_idx, 1);
        assert_eq!(state.space_cursor_idx, 1); // cursor unchanged by select_space
        assert!(state.filter_assignee_id.is_none());
        assert_eq!(state.project_cursor_idx, 0);
        assert_eq!(state.selected_issue_idx, 0);
        assert!(!state.search_active);
        assert!(state.search_query.is_empty());
        assert_eq!(state.screen, Screen::ProjectSelect);
    }

    #[test]
    fn test_comments_loaded_sets_detail_comments() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.detail_issue = Some(make_issue("PROJ-1"));
        state.handle_event(AppEvent::CommentsLoaded {
            issue_key: "PROJ-1".to_string(),
            comments: vec![],
        });
        assert!(state.detail_comments.is_some());
    }

    #[test]
    fn test_comments_loaded_wrong_key_ignored() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.detail_issue = Some(make_issue("PROJ-1"));
        state.handle_event(AppEvent::CommentsLoaded {
            issue_key: "PROJ-99".to_string(),
            comments: vec![],
        });
        assert!(state.detail_comments.is_none());
    }

    #[test]
    fn test_issue_detail_loaded_resets_comments() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.detail_comments = Some(vec![]);
        state.handle_event(AppEvent::IssueDetailLoaded(make_issue("PROJ-1")));
        assert!(state.detail_comments.is_none());
    }

    #[test]
    fn test_select_space_resets_comments() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config, false);
        state.detail_comments = Some(vec![]);
        state.select_space(0);
        assert!(state.detail_comments.is_none());
    }
}
