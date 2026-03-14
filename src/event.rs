use crate::api::models::{Issue, User};
use crossterm::event::KeyEvent;

pub enum AppEvent {
    /// Keyboard input from the crossterm reader thread
    Key(KeyEvent),
    /// Issue list fetched for a space
    IssuesLoaded { space: String, issues: Vec<Issue> },
    /// Single issue detail fetched
    IssueDetailLoaded(Issue),
    /// All users for a space fetched and deduplicated by user.id
    SpaceUsersLoaded { space: String, users: Vec<User> },
    /// Any API error
    ApiError { space: String, message: String },
}
