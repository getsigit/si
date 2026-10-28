//! The HTTP transport every endpoint module is built on.

use {
    crate::{
        environment::{normalize_host, Environment},
        error::{ApiError, ErrorResponse, Result},
        CLIENT_HEADER,
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
    client_name: Option<String>,
    token: Option<String>,
}

/// Builds a [`Client`]. Get one from [`Client::builder`].
#[derive(Debug)]
pub struct ClientBuilder {
    environment: Environment,
    http: Option<reqwest::Client>,
    user_agent: Option<String>,
    base_url: Option<String>,
    client_name: Option<String>,
    token: Option<String>,
}

impl ClientBuilder {
    /// Use this `reqwest::Client` (and so its connection pool, TLS backend and
    /// user agent) instead of building one. Required for HTTPS when the crate
    /// is compiled without a TLS feature.
    pub fn http_client(mut self, http: reqwest::Client) -> Self {
        self.http = Some(http);
        self
    }

    /// The `User-Agent` of the client the crate builds. Ignored when
    /// [`ClientBuilder::http_client`] is set.
    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = Some(user_agent.to_string());
        self
    }

    /// An explicit origin such as `https://sigit.si` or `localhost:4000`. A
    /// missing scheme takes the environment's default and a trailing slash is
    /// dropped. It takes the place of the environment's defaults, so
    /// `SIGIT_HOST` has no effect.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Name this client in the [`CLIENT_HEADER`] header on every request.
    pub fn client_name(mut self, name: &str) -> Self {
        self.client_name = Some(name.to_string());
        self
    }

    /// The bearer token sent on every request.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn build(self) -> Result<Client> {
        let environment = self.environment;
        let base_url = match self.base_url {
            Some(url) => normalize_host(url.trim(), environment.default_scheme()),
            None => environment.base_url(),
        };

        let http = match self.http {
            Some(http) => http,
            None => reqwest::Client::builder()
                .user_agent(self.user_agent.as_deref().unwrap_or(USER_AGENT))
                .build()
                .map_err(|source| ApiError::Transport {
                    host: base_url.clone(),
                    source,
                })?,
        };

        Ok(Client {
            http,
            environment,
            api_url: format!("{base_url}/api/v1"),
            base_url,
            client_name: self.client_name,
            token: self.token,
        })
    }
}

impl Client {
    /// An unauthenticated client. Every endpoint on this API needs a token, so
    /// this is mostly a starting point for [`Client::with_token`].
    ///
    /// Shorthand for `Client::builder(environment).build()`.
    pub fn new(environment: Environment) -> Result<Self> {
        Self::builder(environment).build()
    }

    /// Start configuring a client: base URL, user agent, client name, or your
    /// own `reqwest::Client`.
    pub fn builder(environment: Environment) -> ClientBuilder {
        ClientBuilder {
            environment,
            http: None,
            user_agent: None,
            base_url: None,
            client_name: None,
            token: None,
        }
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
        let mut builder = self.http.request(method, url);
        if let Some(name) = &self.client_name {
            builder = builder.header(CLIENT_HEADER, name);
        }
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
    fn an_explicit_base_url_replaces_the_environment_default() {
        let client = Client::builder(Environment::Production)
            .base_url("127.0.0.1:3000/")
            .build()
            .unwrap();

        assert_eq!(client.base_url(), "https://127.0.0.1:3000");
        assert_eq!(client.api_url(), "https://127.0.0.1:3000/api/v1");
    }

    #[test]
    fn a_bare_base_url_takes_the_environment_scheme() {
        let client = Client::builder(Environment::Dev)
            .base_url("127.0.0.1:3000")
            .build()
            .unwrap();
        assert_eq!(client.api_url(), "http://127.0.0.1:3000/api/v1");
    }

    #[test]
    fn requests_carry_the_client_name_and_token() {
        let client = Client::builder(Environment::Production)
            .base_url("https://sigit.si")
            .client_name("example.app")
            .token("tok")
            .build()
            .unwrap();
        let request = client.request(Method::DELETE, "/me").build().unwrap();
        assert_eq!(request.url().as_str(), "https://sigit.si/api/v1/me");
        assert_eq!(request.method(), Method::DELETE);
        assert_eq!(request.headers()[CLIENT_HEADER], "example.app");
        assert_eq!(request.headers()["authorization"], "Bearer tok");
    }

    #[test]
    fn requests_omit_the_client_header_by_default() {
        let client = Client::new(Environment::Production).unwrap();
        let request = client.request(Method::GET, "user").build().unwrap();
        assert!(request.headers().get(CLIENT_HEADER).is_none());
    }

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
