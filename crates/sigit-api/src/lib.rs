//! A typed client for the [sigit.si](https://sigit.si) JSON API.
//!
//! ```no_run
//! use sigit_api::{Client, Environment, issues::State};
//!
//! # async fn example() -> sigit_api::Result<()> {
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

pub mod account;
pub mod billing;
pub mod client;
pub mod code;
pub mod environment;
pub mod error;
pub mod issues;
pub mod mcp;
pub mod models;
pub mod pulls;
pub mod repos;
pub mod sessions;

pub use {
    client::Client,
    environment::Environment,
    error::{ApiError, ErrorResponse, Result},
    issues::State,
    repos::NewRepository,
};
