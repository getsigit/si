//! The HTTP transport every endpoint module is built on.

use {
    crate::{
        environment::Environment,
        error::{ApiError, ErrorResponse, Result},
    },
    reqwest::{Method, RequestBuilder, StatusCode},
    serde::{de::DeserializeOwned, Serialize},
    tracing::debug,
};

const USER_AGENT: &str = concat!("si/", env!("CARGO_PKG_VERSION"));

/// A configured client for one sigit.si environment.
///
/// Cheap to clone (the inner `reqwest::Client` is an `Arc` over a shared
/// connection pool), so pass it around by value.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    environment: Environment,
    api_url: String,
    base_url: String,
    token: Option<String>,
}

impl Client {
    /// An unauthenticated client. Every endpoint on this API needs a token, so
    /// this is mostly a starting point for [`Client::with_token`].
    pub fn new(environment: Environment) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .map_err(|source| ApiError::Transport {
                host: environment.base_url(),
                source,
            })?;

        Ok(Self {
            http,
            environment,
            api_url: environment.api_url(),
            base_url: environment.base_url(),
            token: None,
        })
    }

    /// Attach the bearer token sent on every subsequent request.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn environment(&self) -> Environment {
        self.environment
    }

    /// The web origin — the base for browser and `.git` clone URLs.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    // ── Verb helpers ────────────────────────────────────────────────────────

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.send(self.request(Method::GET, path)).await
    }

    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        self.send(self.request(Method::POST, path).json(body)).await
    }

    pub async fn patch<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        self.send(self.request(Method::PATCH, path).json(body))
            .await
    }

    /// For endpoints that answer `204 No Content` (sign-out, deletes).
    pub async fn delete(&self, path: &str) -> Result<()> {
        self.send_empty(self.request(Method::DELETE, path)).await
    }

    /// Escape hatch for `si api` — an arbitrary method and path against the API
    /// root, returning the raw JSON body.
    pub async fn raw(
        &self,
        method: Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let mut builder = self.request(method, path);
        if let Some(body) = body {
            builder = builder.json(&body);
        }

        let response = self.execute(builder).await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        // A raw call reports the body as-is, whether or not the status is 2xx —
        // seeing the server's actual error JSON is the point of the command.
        // Only auth failures are translated, so the caller can clear the token.
        if status == StatusCode::UNAUTHORIZED {
            return Err(ApiError::Unauthorized);
        }
        if text.trim().is_empty() {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| ApiError::Parse(format!("{e}: {text}")))
    }

    // ── Plumbing ────────────────────────────────────────────────────────────

    /// Build a request against `<api_url>/<path>`, carrying the bearer token.
    pub fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = format!("{}/{}", self.api_url, path.trim_start_matches('/'));
        let builder = self.http.request(method, url);
        match &self.token {
            Some(token) => builder.bearer_auth(token),
            None => builder,
        }
    }

    /// Send a request and deserialize a JSON body.
    pub async fn send<T: DeserializeOwned>(&self, builder: RequestBuilder) -> Result<T> {
        let response = self.execute(builder).await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(Self::error_from(status, &text));
        }

        serde_json::from_str(&text).map_err(|e| ApiError::Parse(format!("{e}: {text}")))
    }

    /// Send a request that returns no useful body (`204`, or a `200` we ignore).
    pub async fn send_empty(&self, builder: RequestBuilder) -> Result<()> {
        let response = self.execute(builder).await?;
        let status = response.status();

        if status.is_success() {
            return Ok(());
        }

        let text = response.text().await.unwrap_or_default();
        Err(Self::error_from(status, &text))
    }

    async fn execute(&self, builder: RequestBuilder) -> Result<reqwest::Response> {
        if self.token.is_none() {
            debug!("sending an unauthenticated request to {}", self.api_url);
        }

        builder.send().await.map_err(|source| ApiError::Transport {
            host: self.base_url.clone(),
            source,
        })
    }

    /// Turn a non-2xx response into the most specific error we can. The body is
    /// usually an `ErrorResponse`; when it isn't (a proxy's HTML 502, say), fall
    /// back to the status line so the message is still actionable.
    fn error_from(status: StatusCode, body: &str) -> ApiError {
        if status == StatusCode::UNAUTHORIZED {
            return ApiError::Unauthorized;
        }

        match serde_json::from_str::<ErrorResponse>(body) {
            Ok(error) => ApiError::Status {
                status: status.as_u16(),
                error_code: error.error_code,
                message: error.message,
            },
            Err(_) => ApiError::Status {
                status: status.as_u16(),
                error_code: crate::error::codes::UNKNOWN,
                message: format!(
                    "The server returned {}{}",
                    status,
                    truncate_for_message(body)
                ),
            },
        }
    }
}

/// Append a short excerpt of an unparseable body, so an HTML error page shows
/// something useful without dumping a full page into the terminal.
fn truncate_for_message(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        return String::new();
    }

    const MAX: usize = 200;
    let excerpt: String = body.chars().take(MAX).collect();
    if body.chars().count() > MAX {
        format!(": {excerpt}…")
    } else {
        format!(": {excerpt}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_maps_to_the_dedicated_variant() {
        let error = Client::error_from(StatusCode::UNAUTHORIZED, r#"{"error_code":100,"m":"x"}"#);
        assert!(matches!(error, ApiError::Unauthorized));
    }

    #[test]
    fn error_bodies_keep_the_servers_message_and_code() {
        let body = r#"{"error_code":101,"message":"Repository not found."}"#;
        match Client::error_from(StatusCode::NOT_FOUND, body) {
            ApiError::Status {
                status,
                error_code,
                message,
            } => {
                assert_eq!(status, 404);
                assert_eq!(error_code, 101);
                assert_eq!(message, "Repository not found.");
            }
            other => panic!("expected a Status error, got {other:?}"),
        }
    }

    #[test]
    fn non_json_bodies_fall_back_to_the_status_line() {
        match Client::error_from(StatusCode::BAD_GATEWAY, "<html>nginx</html>") {
            ApiError::Status { message, .. } => {
                assert!(message.contains("502"), "{message}");
                assert!(message.contains("nginx"), "{message}");
            }
            other => panic!("expected a Status error, got {other:?}"),
        }
    }

    #[test]
    fn long_html_bodies_are_truncated() {
        let body = "x".repeat(1000);
        let message = truncate_for_message(&body);
        assert!(message.ends_with('…'));
        assert!(message.chars().count() < 250);
    }
}
