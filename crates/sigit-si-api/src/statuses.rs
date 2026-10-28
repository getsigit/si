//! Commit statuses — how CI reports pass or fail onto a commit.

use {
    crate::{
        client::Client,
        error::Result,
        models::{CombinedStatus, CommitStatus},
    },
    reqwest::{Method, RequestBuilder},
};

/// Fields accepted by `POST /api/v1/repos/:owner/:repo/statuses/:sha`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NewStatus {
    /// `pending`, `success`, `failure` or `error`.
    pub state: String,
    /// Which check this is. The server uses `default` when it is left out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_url: Option<String>,
}

impl NewStatus {
    pub fn new(state: impl Into<String>) -> Self {
        Self {
            state: state.into(),
            context: None,
            description: None,
            target_url: None,
        }
    }

    pub fn context(mut self, context: Option<String>) -> Self {
        self.context = context.filter(|c| !c.trim().is_empty());
        self
    }

    pub fn description(mut self, description: Option<String>) -> Self {
        self.description = description.filter(|d| !d.trim().is_empty());
        self
    }

    pub fn target_url(mut self, target_url: Option<String>) -> Self {
        self.target_url = target_url.filter(|u| !u.trim().is_empty());
        self
    }
}

impl Client {
    /// `POST /api/v1/repos/:owner/:repo/statuses/:sha` — report a status on a
    /// commit. Owner only. Reporting the same context again replaces it.
    pub async fn create_status(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        status: &NewStatus,
    ) -> Result<CommitStatus> {
        self.send(self.create_status_request(owner, repo, sha, status))
            .await
    }

    /// `GET /api/v1/repos/:owner/:repo/commits/:sha/status` — the combined
    /// state of a commit and every context behind it.
    pub async fn combined_status(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<CombinedStatus> {
        self.get(&format!("repos/{owner}/{repo}/commits/{sha}/status"))
            .await
    }

    fn create_status_request(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        status: &NewStatus,
    ) -> RequestBuilder {
        self.request(
            Method::POST,
            &format!("repos/{owner}/{repo}/statuses/{sha}"),
        )
        .json(status)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::Environment};

    fn client() -> Client {
        Client::builder(Environment::Production)
            .base_url("https://sigit.si")
            .token("tok")
            .build()
            .unwrap()
    }

    #[test]
    fn a_status_is_posted_to_the_commit() {
        let status = NewStatus::new("success").context(Some("ci/test".into()));
        let request = client()
            .create_status_request("seto", "demo", "abc123", &status)
            .build()
            .unwrap();

        assert_eq!(request.method(), Method::POST);
        assert_eq!(
            request.url().as_str(),
            "https://sigit.si/api/v1/repos/seto/demo/statuses/abc123"
        );
        let body = request.body().and_then(|b| b.as_bytes()).unwrap();
        let body: serde_json::Value = serde_json::from_slice(body).unwrap();
        assert_eq!(body["state"], "success");
        assert_eq!(body["context"], "ci/test");
        assert!(body.get("description").is_none());
        assert!(body.get("target_url").is_none());
    }

    #[test]
    fn blank_optional_fields_are_left_out() {
        let status = NewStatus::new("pending")
            .context(Some(" ".into()))
            .description(Some(String::new()))
            .target_url(None);
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json, serde_json::json!({ "state": "pending" }));
    }

    #[test]
    fn a_combined_status_parses() {
        let body = r#"{"sha":"abc","state":"success","total_count":1,"statuses":[
            {"id":1,"sha":"abc","state":"success","context":"ci","description":null,
             "target_url":null,"created_at":"2026-10-01T10:00:00Z","updated_at":"2026-10-01T10:00:00Z"}]}"#;
        let combined: CombinedStatus = serde_json::from_str(body).unwrap();
        assert_eq!(combined.total_count, 1);
        assert_eq!(combined.statuses[0].context, "ci");
    }
}
