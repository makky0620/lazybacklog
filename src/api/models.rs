#![allow(dead_code)]
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub id: i64,
    #[serde(rename = "projectKey")]
    pub project_key: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueStatus {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Priority {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueType {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Comment {
    pub id: i64,
    pub content: Option<String>,
    #[serde(rename = "createdUser")]
    pub created_user: Option<User>,
    pub created: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Issue {
    pub id: i64,
    #[serde(rename = "issueKey")]
    pub issue_key: String,
    pub summary: String,
    pub description: Option<String>,
    pub assignee: Option<User>,
    pub status: IssueStatus,
    pub priority: Option<Priority>,
    #[serde(rename = "issueType")]
    pub issue_type: Option<IssueType>,
    #[serde(rename = "dueDate")]
    pub due_date: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_issue_with_renamed_fields() {
        let json = serde_json::json!({
            "id": 1,
            "issueKey": "PROJ-1",
            "summary": "Test",
            "description": "desc",
            "assignee": { "id": 10, "name": "Alice" },
            "status": { "id": 1, "name": "Open" },
            "priority": { "id": 2, "name": "Normal" },
            "issueType": { "id": 3, "name": "Bug" },
            "dueDate": "2026-04-01T00:00:00Z"
        });
        let issue: Issue = serde_json::from_value(json).unwrap();
        assert_eq!(issue.issue_key, "PROJ-1");
        assert_eq!(issue.assignee.unwrap().name, "Alice");
        assert_eq!(issue.issue_type.unwrap().name, "Bug");
        assert_eq!(issue.due_date.unwrap(), "2026-04-01T00:00:00Z");
    }

    #[test]
    fn test_deserialize_issue_with_null_optional_fields() {
        let json = serde_json::json!({
            "id": 2,
            "issueKey": "PROJ-2",
            "summary": "Minimal",
            "description": null,
            "assignee": null,
            "status": { "id": 1, "name": "Open" },
            "priority": null,
            "issueType": null,
            "dueDate": null
        });
        let issue: Issue = serde_json::from_value(json).unwrap();
        assert!(issue.assignee.is_none());
        assert!(issue.description.is_none());
        assert!(issue.due_date.is_none());
    }

    #[test]
    fn test_deserialize_project_key_renamed() {
        let json = serde_json::json!({ "id": 100, "projectKey": "PROJ", "name": "My Project" });
        let project: Project = serde_json::from_value(json).unwrap();
        assert_eq!(project.project_key, "PROJ");
    }

    #[test]
    fn test_deserialize_comment() {
        let json = serde_json::json!({
            "id": 1,
            "content": "Hello",
            "createdUser": { "id": 10, "name": "Alice" },
            "created": "2026-03-31T12:00:00Z"
        });
        let comment: Comment = serde_json::from_value(json).unwrap();
        assert_eq!(comment.id, 1);
        assert_eq!(comment.content.as_deref(), Some("Hello"));
        assert_eq!(comment.created_user.unwrap().name, "Alice");
        assert_eq!(comment.created, "2026-03-31T12:00:00Z");
    }

    #[test]
    fn test_deserialize_comment_null_fields() {
        let json = serde_json::json!({
            "id": 2,
            "content": null,
            "createdUser": null,
            "created": "2026-03-31T12:00:00Z"
        });
        let comment: Comment = serde_json::from_value(json).unwrap();
        assert!(comment.content.is_none());
        assert!(comment.created_user.is_none());
    }
}
