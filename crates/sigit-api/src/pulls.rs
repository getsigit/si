//! Pull requests. Served by the MCP tools; see [`crate::mcp`] for why.

use {
    crate::{
        client::Client,
        error::Result,
        issues::State,
        models::{CreatedRef, PullRequest, PullRequestSummary},
    },
    serde_json::json,
};

impl Client {
    /// Pull requests in `owner/repo`.
    pub async fn pulls(&self, repo: &str, state: State) -> Result<Vec<PullRequestSummary>> {
        let args = json!({ "repo": repo, "state": state.as_str() });
        self.call_tool_as("list_pull_requests", args).await
    }

    /// One pull request with its comments and unified diff. The server
    /// truncates diffs past 100 kB and flags it on the result.
    pub async fn pull(&self, repo: &str, number: i64) -> Result<PullRequest> {
        self.call_tool_as(
            "get_pull_request",
            json!({ "repo": repo, "number": number }),
        )
        .await
    }

    /// Open a pull request from `head` into `base`.
    pub async fn create_pull(
        &self,
        repo: &str,
        title: &str,
        head: &str,
        base: &str,
        body: Option<&str>,
    ) -> Result<CreatedRef> {
        let mut args = json!({
            "repo": repo,
            "title": title,
            "head": head,
            "base": base,
        });
        if let Some(body) = body {
            args["body"] = body.into();
        }
        self.call_tool_as("create_pull_request", args).await
    }
}
