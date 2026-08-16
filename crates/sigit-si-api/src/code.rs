//! Reading and searching repository contents, and repo discovery across all
//! repositories the user can see. Served by the MCP tools; see [`crate::mcp`].

use {
    crate::{
        client::Client,
        error::Result,
        models::{CodeHit, FileContents, RepositorySummary},
    },
    serde_json::json,
};

impl Client {
    /// Search repositories by an owner/name substring. Unlike
    /// [`Client::repos`], this spans every repo the user can see, not only the
    /// ones they own.
    pub async fn search_repos(
        &self,
        query: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<RepositorySummary>> {
        let mut args = json!({});
        if let Some(query) = query {
            args["query"] = query.into();
        }
        if let Some(limit) = limit {
            args["limit"] = limit.into();
        }
        self.call_tool_as("list_repositories", args).await
    }

    /// Read one file at a ref, defaulting to the repo's default branch.
    pub async fn file_contents(
        &self,
        repo: &str,
        path: &str,
        git_ref: Option<&str>,
    ) -> Result<FileContents> {
        let mut args = json!({ "repo": repo, "path": path });
        if let Some(git_ref) = git_ref {
            args["ref"] = git_ref.into();
        }
        self.call_tool_as("get_file_contents", args).await
    }

    /// Fixed-string, case-insensitive search across a repository's tracked
    /// files at a ref.
    pub async fn search_code(
        &self,
        repo: &str,
        query: &str,
        git_ref: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<CodeHit>> {
        let mut args = json!({ "repo": repo, "query": query });
        if let Some(git_ref) = git_ref {
            args["ref"] = git_ref.into();
        }
        if let Some(limit) = limit {
            args["limit"] = limit.into();
        }
        self.call_tool_as("search_code", args).await
    }
}
