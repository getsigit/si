//! Issues. Served by the MCP tools; see [`crate::mcp`] for why.

use {
    crate::{
        client::Client,
        error::Result,
        models::{CreatedRef, Issue, IssueSummary},
    },
    serde_json::json,
    std::fmt,
};

/// Which issues or pull requests to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Open,
    Closed,
    Merged,
    All,
}

impl State {
    pub fn as_str(&self) -> &'static str {
        match self {
            State::Open => "open",
            State::Closed => "closed",
            State::Merged => "merged",
            State::All => "all",
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Client {
    /// Issues in `owner/repo`, optionally filtered by a title/body substring.
    pub async fn issues(
        &self,
        repo: &str,
        state: State,
        query: Option<&str>,
    ) -> Result<Vec<IssueSummary>> {
        let mut args = json!({ "repo": repo, "state": state.as_str() });
        if let Some(query) = query {
            args["query"] = query.into();
        }
        self.call_tool_as("list_issues", args).await
    }

    /// One issue with its comments.
    pub async fn issue(&self, repo: &str, number: i64) -> Result<Issue> {
        self.call_tool_as("get_issue", json!({ "repo": repo, "number": number }))
            .await
    }

    /// Open a new issue.
    pub async fn create_issue(
        &self,
        repo: &str,
        title: &str,
        body: Option<&str>,
    ) -> Result<CreatedRef> {
        let mut args = json!({ "repo": repo, "title": title });
        if let Some(body) = body {
            args["body"] = body.into();
        }
        self.call_tool_as("create_issue", args).await
    }

    /// Comment on an issue *or* a pull request — they share one number space.
    pub async fn add_comment(&self, repo: &str, number: i64, body: &str) -> Result<CommentRef> {
        let args = json!({ "repo": repo, "number": number, "body": body });
        self.call_tool_as("add_issue_comment", args).await
    }
}

/// What `add_issue_comment` returns.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CommentRef {
    pub id: i64,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_use_the_wire_spelling_the_server_expects() {
        assert_eq!(State::Open.as_str(), "open");
        assert_eq!(State::Merged.as_str(), "merged");
        assert_eq!(State::default(), State::Open);
    }
}
