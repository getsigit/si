//! A repository's default branch, published as a static site.

use {
    crate::{client::Client, error::Result, models::Site},
    reqwest::{Method, RequestBuilder},
};

impl Client {
    /// `GET /api/v1/repos/:owner/:repo/site` — `404` when the repository has no
    /// site.
    pub async fn site(&self, owner: &str, repo: &str) -> Result<Site> {
        self.get(&site_path(owner, repo)).await
    }

    /// `POST /api/v1/repos/:owner/:repo/site` — publishes the site, or
    /// publishes it again. Owner only. Publishing runs in the background, so
    /// the answer has state `publishing`; poll [`Client::site`] for `live` or
    /// `failed`. Publishing too often answers `429`.
    pub async fn publish_site(&self, owner: &str, repo: &str) -> Result<Site> {
        self.send(self.publish_site_request(owner, repo)).await
    }

    /// `DELETE /api/v1/repos/:owner/:repo/site` — takes the site offline. The
    /// server does it in the background.
    pub async fn unpublish_site(&self, owner: &str, repo: &str) -> Result<()> {
        self.delete(&site_path(owner, repo)).await
    }

    fn publish_site_request(&self, owner: &str, repo: &str) -> RequestBuilder {
        self.request(Method::POST, &site_path(owner, repo))
            .json(&serde_json::json!({}))
    }
}

fn site_path(owner: &str, repo: &str) -> String {
    format!("repos/{owner}/{repo}/site")
}

#[cfg(test)]
mod tests {
    use {super::*, crate::Environment};

    #[test]
    fn publishing_posts_to_the_site_path() {
        let client = Client::builder(Environment::Production)
            .base_url("https://sigit.si")
            .token("tok")
            .build()
            .unwrap();
        let request = client.publish_site_request("seto", "demo").build().unwrap();

        assert_eq!(request.method(), Method::POST);
        assert_eq!(
            request.url().as_str(),
            "https://sigit.si/api/v1/repos/seto/demo/site"
        );
    }

    #[test]
    fn a_site_parses() {
        let body = r#"{"url":"https://demo.example","state":"live","error":null,
            "last_published_at":"2026-10-01T10:00:00Z","files":4,"bytes":2048}"#;
        let site: Site = serde_json::from_str(body).unwrap();
        assert_eq!(site.state, "live");
        assert_eq!(site.files, 4);
    }

    #[test]
    fn a_site_still_publishing_has_no_timestamp_yet() {
        let body = r#"{"url":"https://demo.example","state":"publishing","error":null,
            "last_published_at":null,"files":0,"bytes":0}"#;
        let site: Site = serde_json::from_str(body).unwrap();
        assert!(site.last_published_at.is_none());
    }
}
