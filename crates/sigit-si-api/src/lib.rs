//! A typed client for the [sigit.si](https://sigit.si) JSON API.
//!
//! ```no_run
//! use sigit_si_api::{Client, Environment, issues::State};
//!
//! # async fn example() -> sigit_si_api::Result<()> {
//! let client = Client::new(Environment::Production)?.with_token("…");
//!
//! let me = client.user().await?;
//! let repos = client.repos().await?;
//! let open = client.pulls("seto/sigit-si", State::Open).await?;
//! # Ok(())
//! # }
//! ```
//!
//! Two transports sit behind one [`Client`]. Accounts, repositories, hooks,
//! Cloud Sessions, and billing are plain REST under `/api/v1`. Issues, pull
//! requests, and code search have no REST surface yet, so they go over the
//! JSON-RPC MCP endpoint at `/api/v1/mcp` — that split is invisible at the call
//! site, and can disappear later without breaking callers.

//!
//! # Features
//!
//! - `client` (default): the HTTP [`Client`] and the endpoint modules.
//! - `rustls-tls-native-roots` (default), `rustls-tls-webpki-roots`,
//!   `native-tls`: the TLS backend of the `reqwest::Client` the crate builds
//!   itself. With none of them enabled, HTTPS requests fail unless you pass
//!   your own client to [`client::ClientBuilder::http_client`].
//!
//! With `default-features = false` only the wire types remain ([`models`],
//! [`ErrorResponse`], [`error::codes`], [`Environment`], [`CLIENT_HEADER`]),
//! with no HTTP stack. That suits code that serves or proxies this API.

#[cfg(feature = "client")]
pub mod account;
#[cfg(feature = "client")]
pub mod agent_runs;
#[cfg(feature = "client")]
pub mod billing;
#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "client")]
pub mod code;
pub mod environment;
pub mod error;
#[cfg(feature = "client")]
pub mod issues;
#[cfg(feature = "client")]
pub mod mcp;
pub mod models;
#[cfg(feature = "client")]
pub mod pulls;
#[cfg(feature = "client")]
pub mod repos;
#[cfg(feature = "client")]
pub mod sessions;
#[cfg(feature = "client")]
pub mod statuses;

/// Request header a client uses to name itself. [`client::ClientBuilder::client_name`]
/// sends it on every request.
pub const CLIENT_HEADER: &str = "x-sigit-client";

pub use {environment::Environment, error::ErrorResponse};

#[cfg(feature = "client")]
pub use {
    client::Client,
    error::{ApiError, Result},
    issues::State,
    repos::NewRepository,
    statuses::NewStatus,
};
