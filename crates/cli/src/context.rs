//! Shared state threaded through every command: the API client, the `--json`
//! flag, and repository resolution from the working directory.

use {
    crate::git::{self, RepoSlug},
    anyhow::{anyhow, Result},
    sigit_si_api::{Client, Environment},
};

pub struct Ctx {
    pub client: Client,
    pub environment: Environment,
    pub json: bool,
}

impl Ctx {
    pub fn new(client: Client, environment: Environment, json: bool) -> Self {
        Self {
            client,
            environment,
            json,
        }
    }

    /// Every endpoint but sign-in needs a token; fail fast with an actionable
    /// message instead of letting the request round-trip to a 401.
    pub fn require_token(&self) -> Result<()> {
        if self.client.token().is_some() {
            Ok(())
        } else {
            Err(anyhow!("You are not signed in. Run `si auth login` first."))
        }
    }

    /// Resolve a repository argument: an explicit `OWNER/NAME`, or — when
    /// omitted — the sigit.si remote of the repo in the current directory.
    pub fn resolve_repo(&self, explicit: Option<String>) -> Result<RepoSlug> {
        match explicit {
            Some(input) => RepoSlug::parse(&input),
            None => {
                let cwd = std::env::current_dir()
                    .map_err(|e| anyhow!("Could not read the current directory: {e}"))?;
                git::current_repo(&cwd, self.client.base_url())
            }
        }
    }
}
