use crate::api::models::{Comment, Issue, IssueStatus, Project, User};
use crossterm::event::KeyEvent;

pub enum AppEvent {
    /// Keyboard input from the crossterm reader thread
    Key(KeyEvent),
    /// Issue list fetched for a space
    IssuesLoaded { space: String, issues: Vec<Issue> },
    /// Single issue detail fetched
    IssueDetailLoaded(Issue),
    /// Comments for a single issue fetched
    CommentsLoaded { issue_key: String, comments: Vec<Comment> },
    /// All users for a space fetched and deduplicated by user.id
    SpaceUsersLoaded { space: String, users: Vec<User> },
    /// Projects for a space fetched
    ProjectsLoaded {
        space: String,
        projects: Vec<Project>,
    },
    /// Any API error
    ApiError { space: String, message: String },
    /// Statuses for a space fetched
    StatusesLoaded {
        space: String,
        statuses: Vec<IssueStatus>,
    },
}
