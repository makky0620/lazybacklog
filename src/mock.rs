#![allow(dead_code)]
use crate::api::models::{Issue, IssueStatus, IssueType, Priority, Project, User};
use crate::config::{Config, SpaceConfig};

pub fn demo_config() -> Config {
    Config {
        default_space: "demo".to_string(),
        spaces: vec![SpaceConfig {
            name: "demo".to_string(),
            host: "mock".to_string(),
            api_key: "mock".to_string(),
        }],
    }
}

pub fn projects() -> Vec<Project> {
    vec![
        Project {
            id: 1,
            project_key: "DEMO".to_string(),
            name: "Demo Project".to_string(),
        },
        Project {
            id: 2,
            project_key: "SAMPLE".to_string(),
            name: "Sample Project".to_string(),
        },
    ]
}

pub fn users() -> Vec<User> {
    vec![
        User {
            id: 10,
            name: "Alice".to_string(),
        },
        User {
            id: 20,
            name: "Bob".to_string(),
        },
        User {
            id: 30,
            name: "Charlie".to_string(),
        },
    ]
}

pub fn statuses() -> Vec<IssueStatus> {
    vec![
        IssueStatus {
            id: 1,
            name: "Open".to_string(),
        },
        IssueStatus {
            id: 2,
            name: "In Progress".to_string(),
        },
        IssueStatus {
            id: 3,
            name: "Resolved".to_string(),
        },
        IssueStatus {
            id: 4,
            name: "Closed".to_string(),
        },
    ]
}

pub fn issues() -> Vec<Issue> {
    vec![
        Issue {
            id: 1,
            issue_key: "DEMO-1".to_string(),
            summary: "Set up CI pipeline for the project".to_string(),
            description: Some(
                "Configure GitHub Actions to run tests and linting on every PR.".to_string(),
            ),
            assignee: Some(User {
                id: 10,
                name: "Alice".to_string(),
            }),
            status: IssueStatus {
                id: 2,
                name: "In Progress".to_string(),
            },
            priority: Some(Priority {
                id: 2,
                name: "High".to_string(),
            }),
            issue_type: Some(IssueType {
                id: 1,
                name: "Task".to_string(),
            }),
            due_date: Some("2026-04-01T00:00:00Z".to_string()),
        },
        Issue {
            id: 2,
            issue_key: "DEMO-2".to_string(),
            summary: "Fix login page layout on mobile".to_string(),
            description: Some(
                "The login form overflows on small screens. Apply responsive CSS.".to_string(),
            ),
            assignee: Some(User {
                id: 20,
                name: "Bob".to_string(),
            }),
            status: IssueStatus {
                id: 1,
                name: "Open".to_string(),
            },
            priority: Some(Priority {
                id: 3,
                name: "Normal".to_string(),
            }),
            issue_type: Some(IssueType {
                id: 2,
                name: "Bug".to_string(),
            }),
            due_date: None,
        },
        Issue {
            id: 3,
            issue_key: "DEMO-3".to_string(),
            summary: "Write API documentation".to_string(),
            description: None,
            assignee: None,
            status: IssueStatus {
                id: 1,
                name: "Open".to_string(),
            },
            priority: Some(Priority {
                id: 4,
                name: "Low".to_string(),
            }),
            issue_type: Some(IssueType {
                id: 1,
                name: "Task".to_string(),
            }),
            due_date: Some("2026-05-15T00:00:00Z".to_string()),
        },
        Issue {
            id: 4,
            issue_key: "DEMO-4".to_string(),
            summary: "Add dark mode support".to_string(),
            description: Some("Implement a dark theme toggle in the settings panel.".to_string()),
            assignee: Some(User {
                id: 30,
                name: "Charlie".to_string(),
            }),
            status: IssueStatus {
                id: 2,
                name: "In Progress".to_string(),
            },
            priority: Some(Priority {
                id: 3,
                name: "Normal".to_string(),
            }),
            issue_type: Some(IssueType {
                id: 3,
                name: "Feature".to_string(),
            }),
            due_date: None,
        },
        Issue {
            id: 5,
            issue_key: "DEMO-5".to_string(),
            summary: "Upgrade dependencies to latest stable versions".to_string(),
            description: Some("Run cargo update and resolve any breaking changes.".to_string()),
            assignee: Some(User {
                id: 10,
                name: "Alice".to_string(),
            }),
            status: IssueStatus {
                id: 3,
                name: "Resolved".to_string(),
            },
            priority: Some(Priority {
                id: 3,
                name: "Normal".to_string(),
            }),
            issue_type: Some(IssueType {
                id: 1,
                name: "Task".to_string(),
            }),
            due_date: Some("2026-03-20T00:00:00Z".to_string()),
        },
        Issue {
            id: 6,
            issue_key: "DEMO-6".to_string(),
            summary: "Search bar returns no results for Japanese queries".to_string(),
            description: Some("Unicode normalization issue. Needs investigation.".to_string()),
            assignee: None,
            status: IssueStatus {
                id: 1,
                name: "Open".to_string(),
            },
            priority: Some(Priority {
                id: 2,
                name: "High".to_string(),
            }),
            issue_type: Some(IssueType {
                id: 2,
                name: "Bug".to_string(),
            }),
            due_date: None,
        },
        Issue {
            id: 7,
            issue_key: "DEMO-7".to_string(),
            summary: "Export issues to CSV".to_string(),
            description: Some(
                "Allow users to download the current issue list as a CSV file.".to_string(),
            ),
            assignee: Some(User {
                id: 20,
                name: "Bob".to_string(),
            }),
            status: IssueStatus {
                id: 1,
                name: "Open".to_string(),
            },
            priority: Some(Priority {
                id: 4,
                name: "Low".to_string(),
            }),
            issue_type: Some(IssueType {
                id: 3,
                name: "Feature".to_string(),
            }),
            due_date: Some("2026-06-01T00:00:00Z".to_string()),
        },
        Issue {
            id: 8,
            issue_key: "DEMO-8".to_string(),
            summary: "Performance regression in issue list rendering".to_string(),
            description: Some(
                "List scrolling drops below 30fps when more than 200 issues are loaded."
                    .to_string(),
            ),
            assignee: Some(User {
                id: 30,
                name: "Charlie".to_string(),
            }),
            status: IssueStatus {
                id: 2,
                name: "In Progress".to_string(),
            },
            priority: Some(Priority {
                id: 2,
                name: "High".to_string(),
            }),
            issue_type: Some(IssueType {
                id: 2,
                name: "Bug".to_string(),
            }),
            due_date: None,
        },
        Issue {
            id: 9,
            issue_key: "DEMO-9".to_string(),
            summary: "Add keyboard shortcut cheat sheet".to_string(),
            description: None,
            assignee: None,
            status: IssueStatus {
                id: 3,
                name: "Resolved".to_string(),
            },
            priority: Some(Priority {
                id: 4,
                name: "Low".to_string(),
            }),
            issue_type: Some(IssueType {
                id: 1,
                name: "Task".to_string(),
            }),
            due_date: None,
        },
        Issue {
            id: 10,
            issue_key: "DEMO-10".to_string(),
            summary: "Implement email notifications for due dates".to_string(),
            description: Some(
                "Send a daily digest email to assignees with issues due within 3 days.".to_string(),
            ),
            assignee: Some(User {
                id: 10,
                name: "Alice".to_string(),
            }),
            status: IssueStatus {
                id: 1,
                name: "Open".to_string(),
            },
            priority: Some(Priority {
                id: 3,
                name: "Normal".to_string(),
            }),
            issue_type: Some(IssueType {
                id: 3,
                name: "Feature".to_string(),
            }),
            due_date: Some("2026-07-01T00:00:00Z".to_string()),
        },
    ]
}
