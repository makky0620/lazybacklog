use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

use super::models::{Issue, Project, User};

pub struct BacklogClient {
    base_url: String,
    api_key: String,
    http: Client,
}

impl BacklogClient {
    pub fn new(host: String, api_key: String) -> Result<Self> {
        Self::with_base_url(format!("https://{}/api/v2", host), api_key)
    }

    pub fn with_base_url(base_url: String, api_key: String) -> Result<Self> {
        let http = Client::builder().timeout(Duration::from_secs(10)).build()?;
        Ok(Self {
            base_url,
            api_key,
            http,
        })
    }

    pub async fn fetch_issues(
        &self,
        project_id: Option<i64>,
        assignee_id: Option<i64>,
    ) -> Result<Vec<Issue>> {
        let mut params: Vec<(&str, String)> = vec![
            ("apiKey", self.api_key.clone()),
            ("count", "100".to_string()),
        ];
        if let Some(id) = project_id {
            params.push(("projectId[]", id.to_string()));
        }
        if let Some(id) = assignee_id {
            params.push(("assigneeId[]", id.to_string()));
        }
        let resp = self
            .http
            .get(format!("{}/issues", self.base_url))
            .query(&params)
            .send()
            .await
            .context("Failed to connect to Backlog API")?;
        if resp.status() == 401 {
            anyhow::bail!("401 Unauthorized - check your API key");
        }
        resp.error_for_status_ref()
            .context("Backlog API returned an error")?;
        resp.json::<Vec<Issue>>()
            .await
            .context("Failed to parse issues response")
    }

    pub async fn fetch_issue(&self, id_or_key: &str) -> Result<Issue> {
        let resp = self
            .http
            .get(format!("{}/issues/{}", self.base_url, id_or_key))
            .query(&[("apiKey", &self.api_key)])
            .send()
            .await
            .context("Failed to connect to Backlog API")?;
        if resp.status() == 401 {
            anyhow::bail!("401 Unauthorized - check your API key");
        }
        resp.error_for_status_ref()
            .context("Backlog API returned an error")?;
        resp.json::<Issue>()
            .await
            .context("Failed to parse issue response")
    }

    pub async fn fetch_projects(&self) -> Result<Vec<Project>> {
        let resp = self
            .http
            .get(format!("{}/projects", self.base_url))
            .query(&[("apiKey", &self.api_key)])
            .send()
            .await
            .context("Failed to connect to Backlog API")?;
        if resp.status() == 401 {
            anyhow::bail!("401 Unauthorized - check your API key");
        }
        resp.error_for_status_ref()
            .context("Backlog API returned an error")?;
        resp.json::<Vec<Project>>()
            .await
            .context("Failed to parse projects response")
    }

    pub async fn fetch_project_users(&self, project_id: i64) -> Result<Vec<User>> {
        let resp = self
            .http
            .get(format!("{}/projects/{}/users", self.base_url, project_id))
            .query(&[("apiKey", &self.api_key)])
            .send()
            .await
            .context("Failed to connect to Backlog API")?;
        if resp.status() == 401 {
            anyhow::bail!("401 Unauthorized - check your API key");
        }
        resp.error_for_status_ref()
            .context("Backlog API returned an error")?;
        resp.json::<Vec<User>>()
            .await
            .context("Failed to parse users response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn make_client(server: &MockServer) -> BacklogClient {
        BacklogClient::with_base_url(
            format!("{}/api/v2", server.uri()),
            "test_api_key".to_string(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_fetch_issues_success() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/issues"))
            .and(query_param("apiKey", "test_api_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": 1,
                    "issueKey": "PROJ-1",
                    "summary": "Test issue",
                    "description": "Some description",
                    "assignee": { "id": 10, "name": "Alice" },
                    "status": { "id": 1, "name": "In Progress" },
                    "priority": { "id": 2, "name": "Normal" },
                    "issueType": { "id": 1, "name": "Bug" },
                    "dueDate": null
                }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let issues = client.fetch_issues(None, None).await.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_key, "PROJ-1");
        assert_eq!(issues[0].summary, "Test issue");
        assert_eq!(issues[0].assignee.as_ref().unwrap().name, "Alice");
    }

    #[tokio::test]
    async fn test_fetch_issues_with_assignee_filter() {
        let server = MockServer::start().await;
        // reqwest percent-encodes '[' and ']' as '%5B' and '%5D' in query keys.
        // wiremock 0.6's query_param matcher uses url::Url::query_pairs() which
        // percent-decodes keys before comparison, so "assigneeId[]" matches the
        // wire bytes "assigneeId%5B%5D". If this test fails to match, change to
        // query_param("assigneeId%5B%5D", "42").
        Mock::given(method("GET"))
            .and(path("/api/v2/issues"))
            .and(query_param("assigneeId[]", "42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let issues = client.fetch_issues(None, Some(42)).await.unwrap();
        assert_eq!(issues.len(), 0);
    }

    #[tokio::test]
    async fn test_fetch_issues_with_project_filter() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/issues"))
            .and(query_param("projectId[]", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let issues = client.fetch_issues(Some(100), None).await.unwrap();
        assert_eq!(issues.len(), 0);
    }

    #[tokio::test]
    async fn test_fetch_issues_401() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/issues"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let err = client.fetch_issues(None, None).await.unwrap_err();
        assert!(err.to_string().contains("401 Unauthorized"));
    }

    #[tokio::test]
    async fn test_fetch_issue_detail() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/issues/PROJ-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 1,
                "issueKey": "PROJ-1",
                "summary": "Detailed issue",
                "description": "Full description here",
                "assignee": null,
                "status": { "id": 1, "name": "Open" },
                "priority": null,
                "issueType": null,
                "dueDate": "2026-04-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let issue = client.fetch_issue("PROJ-1").await.unwrap();
        assert_eq!(issue.issue_key, "PROJ-1");
        assert_eq!(issue.description.unwrap(), "Full description here");
    }

    #[tokio::test]
    async fn test_fetch_projects_success() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/projects"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "id": 100, "projectKey": "PROJ", "name": "My Project" }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let projects = client.fetch_projects().await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].project_key, "PROJ");
    }

    #[tokio::test]
    async fn test_fetch_project_users() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/projects/100/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "id": 10, "name": "Alice" },
                { "id": 20, "name": "Bob" }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let users = client.fetch_project_users(100).await.unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].name, "Alice");
    }
}
