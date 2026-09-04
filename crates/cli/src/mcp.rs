//! `si` as an MCP server over stdio.
//!
//! `si mcp` stops being a one-shot command and becomes a long-lived
//! [Model Context Protocol](https://modelcontextprotocol.io) server, so an
//! agent can do what the CLI does: list repos, read and search code, triage
//! issues, open pull requests, drive Cloud Sessions.
//!
//! sigit.si already serves a remote MCP endpoint at `/api/v1/mcp` (it's what
//! [`sigit_si_api::mcp`] calls into), so a local server only earns its keep by
//! doing what a remote one can't:
//!
//! - **No auth to configure.** It reuses the token `si auth login` stored, so
//!   an editor points at a binary and is done — no OAuth dance, no PAT pasted
//!   into a config file.
//! - **It knows where you're standing.** Every repo-scoped tool takes `repo`
//!   as an *optional* argument and falls back to the sigit.si remote of the
//!   working directory, the same way the CLI does. `create_pull_request`
//!   likewise defaults `head` to the checked-out branch.
//! - **It reaches the whole CLI.** Cloud Sessions, webhooks, billing, and repo
//!   creation are plain REST that the remote MCP endpoint doesn't expose.
//!
//! Tools call the same [`sigit_si_api`] methods the command handlers do,
//! skipping the `ui` layer entirely: stdout is the JSON-RPC channel, so
//! nothing here may print to it. Logging already goes to stderr (see `main`).
//!
//! Toolsets and `--read-only` follow the convention GitHub's `github-mcp-server`
//! set — trimming the tool list keeps the model's choice sharp and its context
//! small.

use {
    crate::{
        cli::McpArgs,
        context::Ctx,
        git::{self, RepoSlug},
    },
    anyhow::{anyhow, Result},
    rmcp::{
        handler::server::{router::tool::ToolRouter, wrapper::Parameters, ServerHandler},
        model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerInfo},
        tool, tool_handler, tool_router,
        transport::stdio,
        ErrorData, ServiceExt,
    },
    schemars::JsonSchema,
    serde::Deserialize,
    sigit_si_api::{Client, NewRepository, State},
};

/// The tool groups `--toolsets` selects from.
const TOOLSETS: [&str; 8] = [
    "account", "repo", "issue", "pr", "code", "hook", "session", "billing",
];

/// The sigit.si MCP server. Holds the same [`Ctx`] the command handlers get,
/// which is where the API client and working-directory repo resolution live.
pub struct SiMcpServer {
    ctx: Ctx,
    tool_router: ToolRouter<Self>,
}

impl SiMcpServer {
    fn new(ctx: Ctx, tool_router: ToolRouter<Self>) -> Self {
        Self { ctx, tool_router }
    }

    /// The API client, refusing early when there's no token rather than
    /// letting every tool round-trip to a 401.
    fn client(&self) -> Result<&Client, ErrorData> {
        self.ctx
            .require_token()
            .map(|()| &self.ctx.client)
            .map_err(|e| ErrorData::invalid_request(e.to_string(), None))
    }

    /// Resolve a tool's optional `repo` argument, falling back to the repo the
    /// server was started in.
    fn repo(&self, explicit: Option<String>) -> Result<String, ErrorData> {
        self.ctx
            .resolve_repo(explicit)
            .map(|slug| slug.to_string())
            .map_err(|e| ErrorData::invalid_request(e.to_string(), None))
    }
}

/// Serialize a value into a one-block successful tool result.
fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::json(value)?]))
}

fn text_result(message: impl Into<String>) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::text(
        message.into(),
    )]))
}

fn to_error_data(error: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(error.to_string(), None)
}

/// Which issues or pull requests to list. An enum rather than a string so the
/// advertised schema tells the model what the choices are.
#[derive(Debug, Default, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum StateArg {
    #[default]
    Open,
    Closed,
    Merged,
    All,
}

impl From<StateArg> for State {
    fn from(state: StateArg) -> Self {
        match state {
            StateArg::Open => State::Open,
            StateArg::Closed => State::Closed,
            StateArg::Merged => State::Merged,
            StateArg::All => State::All,
        }
    }
}

/// The `repo` argument every repository-scoped tool shares.
#[derive(Debug, Deserialize, JsonSchema)]
struct RepoArgs {
    /// Repository as OWNER/NAME. Defaults to the repository the server is
    /// running in.
    #[serde(default)]
    repo: Option<String>,
}

// ---------------------------------------------------------------- account --

#[tool_router(router = account_router)]
impl SiMcpServer {
    /// Show the signed-in sigit.si user: name, email, and plan.
    #[tool(annotations(read_only_hint = true))]
    async fn whoami(&self) -> Result<CallToolResult, ErrorData> {
        let user = self.client()?.user().await.map_err(to_error_data)?;
        json_result(&user)
    }
}

// ------------------------------------------------------------------- repo --

#[derive(Debug, Deserialize, JsonSchema)]
struct ListRepositoriesArgs {
    /// Case-insensitive substring of the owner or repository name. Omit to
    /// list everything visible.
    #[serde(default)]
    query: Option<String>,
    /// How many repositories to return. Defaults to the server's own limit.
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateRepositoryArgs {
    /// Name for the new repository.
    name: String,
    /// Whether the repository is private. Defaults to public.
    #[serde(default)]
    private: bool,
    /// Optional one-line description.
    #[serde(default)]
    description: Option<String>,
}

#[tool_router(router = repo_router)]
impl SiMcpServer {
    /// Search every repository the user can see by an owner/name substring.
    /// Use this to find the OWNER/NAME other tools take.
    #[tool(annotations(read_only_hint = true))]
    async fn list_repositories(
        &self,
        Parameters(ListRepositoriesArgs { query, limit }): Parameters<ListRepositoriesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let repos = self
            .client()?
            .search_repos(query.as_deref(), limit)
            .await
            .map_err(to_error_data)?;
        json_result(&repos)
    }

    /// List the repositories the signed-in user owns, newest first.
    #[tool(annotations(read_only_hint = true))]
    async fn list_my_repositories(&self) -> Result<CallToolResult, ErrorData> {
        let repos = self.client()?.repos().await.map_err(to_error_data)?;
        json_result(&repos)
    }

    /// Identify the sigit.si repository and branch the server is running in.
    /// Call this to find out what "the current repo" resolves to.
    #[tool(annotations(read_only_hint = true))]
    async fn current_repository(&self) -> Result<CallToolResult, ErrorData> {
        let cwd = std::env::current_dir().map_err(to_error_data)?;
        let slug: RepoSlug = self
            .ctx
            .resolve_repo(None)
            .map_err(|e| ErrorData::invalid_request(e.to_string(), None))?;

        json_result(&serde_json::json!({
            "repo": slug.to_string(),
            "owner": slug.owner,
            "name": slug.name,
            "branch": git::current_branch(&cwd),
            "directory": cwd.display().to_string(),
        }))
    }

    /// Create a new, empty repository to push to.
    #[tool]
    async fn create_repository(
        &self,
        Parameters(CreateRepositoryArgs {
            name,
            private,
            description,
        }): Parameters<CreateRepositoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let new_repo = NewRepository::new(name)
            .private(private)
            .description(description);
        let repo = self
            .client()?
            .create_repo(&new_repo)
            .await
            .map_err(to_error_data)?;
        json_result(&repo)
    }
}

// ------------------------------------------------------------------ issue --

#[derive(Debug, Deserialize, JsonSchema)]
struct ListIssuesArgs {
    /// Repository as OWNER/NAME. Defaults to the repository the server is
    /// running in.
    #[serde(default)]
    repo: Option<String>,
    /// Which issues to list. Defaults to open.
    #[serde(default)]
    state: StateArg,
    /// Optional substring to match against issue titles and bodies.
    #[serde(default)]
    query: Option<String>,
}

/// Shared by the issue and pull request tools, which draw on one numbering.
#[derive(Debug, Deserialize, JsonSchema)]
struct NumberArgs {
    /// The issue or pull request number.
    number: i64,
    /// Repository as OWNER/NAME. Defaults to the repository the server is
    /// running in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateIssueArgs {
    /// Issue title.
    title: String,
    /// Issue body, in Markdown.
    #[serde(default)]
    body: Option<String>,
    /// Repository as OWNER/NAME. Defaults to the repository the server is
    /// running in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddCommentArgs {
    /// The issue or pull request number to comment on.
    number: i64,
    /// Comment body, in Markdown.
    body: String,
    /// Repository as OWNER/NAME. Defaults to the repository the server is
    /// running in.
    #[serde(default)]
    repo: Option<String>,
}

#[tool_router(router = issue_router)]
impl SiMcpServer {
    /// List a repository's issues, optionally filtered by state and a
    /// title/body substring.
    #[tool(annotations(read_only_hint = true))]
    async fn list_issues(
        &self,
        Parameters(ListIssuesArgs { repo, state, query }): Parameters<ListIssuesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let repo = self.repo(repo)?;
        let issues = self
            .client()?
            .issues(&repo, state.into(), query.as_deref())
            .await
            .map_err(to_error_data)?;
        json_result(&issues)
    }

    /// Fetch one issue by number, including its body and comments.
    #[tool(annotations(read_only_hint = true))]
    async fn get_issue(
        &self,
        Parameters(NumberArgs { number, repo }): Parameters<NumberArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let repo = self.repo(repo)?;
        let issue = self
            .client()?
            .issue(&repo, number)
            .await
            .map_err(to_error_data)?;
        json_result(&issue)
    }

    /// Open a new issue.
    #[tool]
    async fn create_issue(
        &self,
        Parameters(CreateIssueArgs { title, body, repo }): Parameters<CreateIssueArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let repo = self.repo(repo)?;
        let created = self
            .client()?
            .create_issue(&repo, &title, body.as_deref())
            .await
            .map_err(to_error_data)?;
        json_result(&created)
    }

    /// Comment on an issue or a pull request — they share one numbering.
    #[tool]
    async fn add_issue_comment(
        &self,
        Parameters(AddCommentArgs { number, body, repo }): Parameters<AddCommentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let repo = self.repo(repo)?;
        let comment = self
            .client()?
            .add_comment(&repo, number, &body)
            .await
            .map_err(to_error_data)?;
        json_result(&comment)
    }
}

// --------------------------------------------------------------------- pr --

#[derive(Debug, Deserialize, JsonSchema)]
struct ListPullsArgs {
    /// Repository as OWNER/NAME. Defaults to the repository the server is
    /// running in.
    #[serde(default)]
    repo: Option<String>,
    /// Which pull requests to list. Defaults to open.
    #[serde(default)]
    state: StateArg,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreatePullArgs {
    /// Pull request title.
    title: String,
    /// Branch to merge from. Defaults to the branch checked out where the
    /// server is running.
    #[serde(default)]
    head: Option<String>,
    /// Branch to merge into. Defaults to main.
    #[serde(default)]
    base: Option<String>,
    /// Pull request body, in Markdown.
    #[serde(default)]
    body: Option<String>,
    /// Repository as OWNER/NAME. Defaults to the repository the server is
    /// running in.
    #[serde(default)]
    repo: Option<String>,
}

#[tool_router(router = pr_router)]
impl SiMcpServer {
    /// List a repository's pull requests, optionally filtered by state.
    #[tool(annotations(read_only_hint = true))]
    async fn list_pull_requests(
        &self,
        Parameters(ListPullsArgs { repo, state }): Parameters<ListPullsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let repo = self.repo(repo)?;
        let pulls = self
            .client()?
            .pulls(&repo, state.into())
            .await
            .map_err(to_error_data)?;
        json_result(&pulls)
    }

    /// Fetch one pull request with its comments and unified diff. Diffs past
    /// 100 kB come back truncated and flagged.
    #[tool(annotations(read_only_hint = true))]
    async fn get_pull_request(
        &self,
        Parameters(NumberArgs { number, repo }): Parameters<NumberArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let repo = self.repo(repo)?;
        let pull = self
            .client()?
            .pull(&repo, number)
            .await
            .map_err(to_error_data)?;
        json_result(&pull)
    }

    /// Open a pull request from one branch into another.
    #[tool]
    async fn create_pull_request(
        &self,
        Parameters(CreatePullArgs {
            title,
            head,
            base,
            body,
            repo,
        }): Parameters<CreatePullArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let repo = self.repo(repo)?;
        let head = match head {
            Some(head) => head,
            None => {
                let cwd = std::env::current_dir().map_err(to_error_data)?;
                git::current_branch(&cwd).ok_or_else(|| {
                    ErrorData::invalid_request(
                        "Could not determine the current branch. Pass `head`.",
                        None,
                    )
                })?
            }
        };
        let base = base.unwrap_or_else(|| "main".to_string());

        let created = self
            .client()?
            .create_pull(&repo, &title, &head, &base, body.as_deref())
            .await
            .map_err(to_error_data)?;
        json_result(&created)
    }
}

// ------------------------------------------------------------------- code --

#[derive(Debug, Deserialize, JsonSchema)]
struct FileContentsArgs {
    /// Path to the file, relative to the repository root.
    path: String,
    /// Branch, tag, or commit SHA. Defaults to the repository's default branch.
    #[serde(default, rename = "ref")]
    git_ref: Option<String>,
    /// Repository as OWNER/NAME. Defaults to the repository the server is
    /// running in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchCodeArgs {
    /// Fixed string to search for. Case-insensitive, not a regex.
    query: String,
    /// Branch, tag, or commit SHA. Defaults to the repository's default branch.
    #[serde(default, rename = "ref")]
    git_ref: Option<String>,
    /// How many matches to return.
    #[serde(default)]
    limit: Option<u32>,
    /// Repository as OWNER/NAME. Defaults to the repository the server is
    /// running in.
    #[serde(default)]
    repo: Option<String>,
}

#[tool_router(router = code_router)]
impl SiMcpServer {
    /// Read a file's contents at a ref.
    #[tool(annotations(read_only_hint = true))]
    async fn get_file_contents(
        &self,
        Parameters(FileContentsArgs {
            path,
            git_ref,
            repo,
        }): Parameters<FileContentsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let repo = self.repo(repo)?;
        let file = self
            .client()?
            .file_contents(&repo, &path, git_ref.as_deref())
            .await
            .map_err(to_error_data)?;
        json_result(&file)
    }

    /// Search a repository's tracked files for a fixed string, returning
    /// matching files with line snippets.
    #[tool(annotations(read_only_hint = true))]
    async fn search_code(
        &self,
        Parameters(SearchCodeArgs {
            query,
            git_ref,
            limit,
            repo,
        }): Parameters<SearchCodeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let repo = self.repo(repo)?;
        let hits = self
            .client()?
            .search_code(&repo, &query, git_ref.as_deref(), limit)
            .await
            .map_err(to_error_data)?;
        json_result(&hits)
    }
}

// ------------------------------------------------------------------- hook --

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateHookArgs {
    /// URL sigit.si should POST to.
    url: String,
    /// Events to subscribe to. Defaults to push.
    #[serde(default)]
    events: Option<Vec<String>>,
    /// Repository as OWNER/NAME. Defaults to the repository the server is
    /// running in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeleteHookArgs {
    /// The webhook's id.
    id: i64,
    /// Repository as OWNER/NAME. Defaults to the repository the server is
    /// running in.
    #[serde(default)]
    repo: Option<String>,
}

#[tool_router(router = hook_router)]
impl SiMcpServer {
    /// List a repository's deploy webhooks.
    #[tool(annotations(read_only_hint = true))]
    async fn list_hooks(
        &self,
        Parameters(RepoArgs { repo }): Parameters<RepoArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let slug = self.slug(repo)?;
        let hooks = self
            .client()?
            .hooks(&slug.owner, &slug.name)
            .await
            .map_err(to_error_data)?;
        json_result(&hooks)
    }

    /// Register a webhook. The response carries the signing secret, which is
    /// shown once and cannot be retrieved again.
    #[tool]
    async fn create_hook(
        &self,
        Parameters(CreateHookArgs { url, events, repo }): Parameters<CreateHookArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let slug = self.slug(repo)?;
        let events = events.unwrap_or_else(|| vec!["push".to_string()]);
        let hook = self
            .client()?
            .create_hook(&slug.owner, &slug.name, &url, &events)
            .await
            .map_err(to_error_data)?;
        json_result(&hook)
    }

    /// Remove a webhook. This cannot be undone.
    #[tool(annotations(destructive_hint = true))]
    async fn delete_hook(
        &self,
        Parameters(DeleteHookArgs { id, repo }): Parameters<DeleteHookArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let slug = self.slug(repo)?;
        self.client()?
            .delete_hook(&slug.owner, &slug.name, id)
            .await
            .map_err(to_error_data)?;
        text_result(format!("Deleted webhook {id} from {slug}."))
    }
}

// ---------------------------------------------------------------- session --

#[derive(Debug, Deserialize, JsonSchema)]
struct SessionIdArgs {
    /// The Cloud Session's id.
    id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateSessionArgs {
    /// Title for the session.
    #[serde(default)]
    title: Option<String>,
    /// Model the session runs on. Defaults to the account's default.
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateSessionArgs {
    /// The Cloud Session's id.
    id: i64,
    /// New title. Omit to leave unchanged.
    #[serde(default)]
    title: Option<String>,
    /// New model. Omit to leave unchanged.
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AppendMessageArgs {
    /// The Cloud Session's id.
    id: i64,
    /// Who the message is from: user or assistant.
    role: String,
    /// The message text.
    content: String,
}

#[tool_router(router = session_router)]
impl SiMcpServer {
    /// List siGit Code Cloud Sessions, most recently updated first.
    #[tool(annotations(read_only_hint = true))]
    async fn list_sessions(&self) -> Result<CallToolResult, ErrorData> {
        let sessions = self.client()?.sessions().await.map_err(to_error_data)?;
        json_result(&sessions)
    }

    /// Fetch one Cloud Session with its full transcript.
    #[tool(annotations(read_only_hint = true))]
    async fn get_session(
        &self,
        Parameters(SessionIdArgs { id }): Parameters<SessionIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let session = self.client()?.session(id).await.map_err(to_error_data)?;
        json_result(&session)
    }

    /// Start a new Cloud Session.
    #[tool]
    async fn create_session(
        &self,
        Parameters(CreateSessionArgs { title, model }): Parameters<CreateSessionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let session = self
            .client()?
            .create_session(title.as_deref(), model.as_deref())
            .await
            .map_err(to_error_data)?;
        json_result(&session)
    }

    /// Rename a Cloud Session or change the model it runs on.
    #[tool]
    async fn update_session(
        &self,
        Parameters(UpdateSessionArgs { id, title, model }): Parameters<UpdateSessionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let session = self
            .client()?
            .update_session(id, title.as_deref(), model.as_deref())
            .await
            .map_err(to_error_data)?;
        json_result(&session)
    }

    /// Append one message to a Cloud Session's transcript.
    #[tool]
    async fn append_session_message(
        &self,
        Parameters(AppendMessageArgs { id, role, content }): Parameters<AppendMessageArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let appended = self
            .client()?
            .append_message(id, &role, &content)
            .await
            .map_err(to_error_data)?;
        json_result(&appended)
    }

    /// Delete a Cloud Session and its transcript. This cannot be undone.
    #[tool(annotations(destructive_hint = true))]
    async fn delete_session(
        &self,
        Parameters(SessionIdArgs { id }): Parameters<SessionIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        self.client()?
            .delete_session(id)
            .await
            .map_err(to_error_data)?;
        text_result(format!("Deleted session {id}."))
    }
}

// ---------------------------------------------------------------- billing --

#[tool_router(router = billing_router)]
impl SiMcpServer {
    /// Show the account's current plan and cloud usage.
    #[tool(annotations(read_only_hint = true))]
    async fn get_billing(&self) -> Result<CallToolResult, ErrorData> {
        let billing = self.client()?.billing().await.map_err(to_error_data)?;
        json_result(&billing)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SiMcpServer {
    fn get_info(&self) -> ServerInfo {
        // `Implementation` is `#[non_exhaustive]`, so start from the build-env
        // default and override the identity fields to report `si`, not `rmcp`.
        let mut server_info = Implementation::from_build_env();
        server_info.name = "si".to_string();
        server_info.version = env!("CARGO_PKG_VERSION").to_string();

        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(server_info)
            .with_instructions(
                "The sigit.si CLI exposed as MCP tools. Authentication uses the token stored \
                 by `si auth login`; tools run non-interactively. Every repository tool takes \
                 `repo` as OWNER/NAME, and omitting it falls back to the sigit.si repository \
                 the server was started in — call `current_repository` to see what that \
                 resolves to. Use `list_repositories` to find the OWNER/NAME of anything else.",
            )
    }
}

impl SiMcpServer {
    /// [`Self::repo`], parsed back into its owner and name halves for the REST
    /// endpoints that take them separately.
    fn slug(&self, explicit: Option<String>) -> Result<RepoSlug, ErrorData> {
        self.ctx
            .resolve_repo(explicit)
            .map_err(|e| ErrorData::invalid_request(e.to_string(), None))
    }
}

/// Assemble the router from the requested toolsets, then drop the writers if
/// `--read-only` was passed.
///
/// An unknown toolset name is an error rather than a silent no-op: a typo in an
/// editor's config would otherwise show up as a server that mysteriously
/// offers fewer tools than expected.
fn build_router(toolsets: &[String], read_only: bool) -> Result<ToolRouter<SiMcpServer>> {
    let selected: Vec<String> = if toolsets.is_empty() {
        TOOLSETS.iter().map(|name| name.to_string()).collect()
    } else {
        toolsets
            .iter()
            .map(|name| name.trim().to_ascii_lowercase())
            .collect()
    };

    let mut router = ToolRouter::new();
    for name in &selected {
        let group = match name.as_str() {
            "account" => SiMcpServer::account_router(),
            "repo" => SiMcpServer::repo_router(),
            "issue" => SiMcpServer::issue_router(),
            "pr" => SiMcpServer::pr_router(),
            "code" => SiMcpServer::code_router(),
            "hook" => SiMcpServer::hook_router(),
            "session" => SiMcpServer::session_router(),
            "billing" => SiMcpServer::billing_router(),
            other => {
                return Err(anyhow!(
                    "Unknown toolset {other:?}. Expected one of: {}.",
                    TOOLSETS.join(", ")
                ))
            }
        };
        router.merge(group);
    }

    if read_only {
        let writers: Vec<String> = router
            .list_all()
            .into_iter()
            .filter(|tool| {
                tool.annotations
                    .as_ref()
                    .and_then(|annotations| annotations.read_only_hint)
                    != Some(true)
            })
            .map(|tool| tool.name.to_string())
            .collect();

        for name in writers {
            router.remove_route(&name);
        }
    }

    Ok(router)
}

/// Run the MCP server over stdio until the client disconnects.
pub async fn run(ctx: Ctx, args: McpArgs) -> Result<()> {
    let router = build_router(&args.toolsets, args.read_only)?;
    tracing::info!(tools = router.list_all().len(), "starting the MCP server");

    let running = SiMcpServer::new(ctx, router)
        .serve(stdio())
        .await
        .map_err(|e| anyhow!("Could not start the MCP server: {e}"))?;

    running
        .waiting()
        .await
        .map_err(|e| anyhow!("The MCP server stopped unexpectedly: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool_names(toolsets: &[String], read_only: bool) -> Vec<String> {
        build_router(toolsets, read_only)
            .unwrap()
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect()
    }

    #[test]
    fn the_default_router_spans_every_toolset() {
        let names = tool_names(&[], false);

        for expected in [
            "whoami",
            "list_repositories",
            "list_issues",
            "list_pull_requests",
            "search_code",
            "list_hooks",
            "list_sessions",
            "get_billing",
        ] {
            assert!(
                names.contains(&expected.to_string()),
                "missing {expected:?}"
            );
        }
    }

    #[test]
    fn a_toolset_selection_leaves_the_rest_out() {
        let names = tool_names(&["issue".to_string(), "pr".to_string()], false);

        assert!(names.contains(&"create_issue".to_string()));
        assert!(names.contains(&"get_pull_request".to_string()));
        assert!(!names.contains(&"get_billing".to_string()));
        assert!(!names.contains(&"list_sessions".to_string()));
    }

    #[test]
    fn read_only_mode_drops_every_writer() {
        let names = tool_names(&[], true);

        assert!(names.contains(&"list_issues".to_string()));
        assert!(names.contains(&"get_file_contents".to_string()));

        for writer in [
            "create_issue",
            "create_pull_request",
            "add_issue_comment",
            "create_repository",
            "create_hook",
            "delete_hook",
            "create_session",
            "delete_session",
        ] {
            assert!(
                !names.contains(&writer.to_string()),
                "read-only mode should have dropped {writer:?}"
            );
        }
    }

    #[test]
    fn an_unknown_toolset_is_rejected() {
        let error = build_router(&["repo".to_string(), "nope".to_string()], false).unwrap_err();
        assert!(error.to_string().contains("nope"));
    }

    #[test]
    fn toolset_names_are_case_and_space_insensitive() {
        assert_eq!(
            tool_names(&[" Billing ".to_string()], false),
            vec!["get_billing".to_string()]
        );
    }
}
