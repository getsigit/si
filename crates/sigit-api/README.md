# sigit-api

A typed Rust client for the [sigit.si](https://sigit.si) JSON API. Used by the
`si` CLI (`crates/cli` in this workspace) and usable standalone by any Rust
project — the desktop app included.

```rust
use sigit_api::{Client, Environment};

let client = Client::new(Environment::Production)?.with_token(token);
let me = client.user().await?;
let repos = client.repos().await?;
```

See the crate-level docs (`cargo doc --open -p sigit-api`) for the full
surface: accounts, repositories, webhooks, Cloud Sessions, billing, and
issues/pull requests/code search (served over an internal MCP JSON-RPC
bridge).
