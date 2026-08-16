//! Repositories and their webhooks.

use crate::{
    client::Client,
    error::Result,
    models::{GitCredential, Hook, Repository},
};

/// Fields accepted by `POST /api/v1/repos`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NewRepository {
    pub name: String,
    pub is_private: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl NewRepository {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_private: false,
            description: None,
        }
    }

    pub fn private(mut self, private: bool) -> Self {
        self.is_private = private;
        self
    }

    pub fn description(mut self, description: Option<String>) -> Self {
        self.description = description.filter(|d| !d.trim().is_empty());
        self
    }
}

impl Client {
    /// `GET /api/v1/repos` — the signed-in user's repositories, newest first.
    pub async fn repos(&self) -> Result<Vec<Repository>> {
        self.get("repos").await
    }

    /// `POST /api/v1/repos` — creates an empty bare repo to push to.
    pub async fn create_repo(&self, repo: &NewRepository) -> Result<Repository> {
        self.post("repos", repo).await
    }

    /// `GET /api/v1/repos/:owner/:repo/hooks`
    pub async fn hooks(&self, owner: &str, repo: &str) -> Result<Vec<Hook>> {
        self.get(&format!("repos/{owner}/{repo}/hooks")).await
    }

    /// `POST /api/v1/repos/:owner/:repo/hooks`
    ///
    /// The response is the only place the signing secret ever appears.
    pub async fn create_hook(
        &self,
        owner: &str,
        repo: &str,
        url: &str,
        events: &[String],
    ) -> Result<Hook> {
        let body = serde_json::json!({ "url": url, "events": events });
        self.post(&format!("repos/{owner}/{repo}/hooks"), &body)
            .await
    }

    /// `DELETE /api/v1/repos/:owner/:repo/hooks/:id`
    pub async fn delete_hook(&self, owner: &str, repo: &str, id: i64) -> Result<()> {
        self.delete(&format!("repos/{owner}/{repo}/hooks/{id}"))
            .await
    }

    /// `POST /api/v1/git_credentials` — mints a 1-hour, read-scoped git
    /// password. This, not the account token, is what git clones with.
    pub async fn git_credential(&self) -> Result<GitCredential> {
        self.post("git_credentials", &serde_json::json!({})).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_description_is_omitted_rather_than_sent_as_empty() {
        let repo = NewRepository::new("demo").description(Some("   ".into()));
        let json = serde_json::to_value(&repo).unwrap();
        assert!(json.get("description").is_none());
    }

    #[test]
    fn a_real_description_is_sent() {
        let repo = NewRepository::new("demo")
            .private(true)
            .description(Some("A demo repo".into()));
        let json = serde_json::to_value(&repo).unwrap();
        assert_eq!(json["description"], "A demo repo");
        assert_eq!(json["is_private"], true);
    }
}
