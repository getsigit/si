//! The `si` command tree.

use {clap::Parser, sigit_si_api::Environment, std::str::FromStr};

#[derive(Parser)]
#[command(name = "si", version, about = "The sigit.si command line interface.")]
pub struct Cli {
    /// Which sigit.si deployment to talk to. Overridden by SIGIT_HOST.
    #[arg(short, long, global = true, env = "SIGIT_ENVIRONMENT", default_value = "production", value_parser = Environment::from_str)]
    pub environment: Environment,

    /// Print machine-readable JSON instead of formatted text, where supported.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Sign in, sign out, and inspect your session.
    #[command(subcommand)]
    Auth(AuthCommand),

    /// Your sigit.si profile.
    Me,

    /// List, create, and inspect repositories.
    #[command(subcommand)]
    Repo(RepoCommand),

    /// List, open, and comment on pull requests.
    #[command(subcommand)]
    Pr(PrCommand),

    /// List, open, and comment on issues.
    #[command(subcommand)]
    Issue(IssueCommand),

    /// Read files and search code in a repository.
    #[command(subcommand)]
    Code(CodeCommand),

    /// Manage a repository's deploy webhooks.
    #[command(subcommand)]
    Hook(HookCommand),

    /// Manage siGit Code Cloud Sessions.
    #[command(subcommand)]
    Session(SessionCommand),

    /// Plan, usage, and the hosted billing portal.
    #[command(subcommand)]
    Billing(BillingCommand),

    /// Open something on sigit.si in your browser.
    Browse(BrowseArgs),

    /// Call the sigit.si API directly. The escape hatch for anything without
    /// a dedicated command.
    Api(ApiArgs),

    /// Serve these commands to an AI agent as MCP tools over stdio.
    Mcp(McpArgs),
}

#[derive(clap::Subcommand)]
pub enum AuthCommand {
    /// Sign in with your sigit.si email and password.
    Login {
        #[arg(long)]
        email: Option<String>,
    },
    /// Sign out and remove the stored token.
    Logout,
    /// Show who is currently signed in.
    Status,
}

#[derive(clap::Subcommand)]
pub enum RepoCommand {
    /// List your repositories.
    List,
    /// Search all repositories you can see.
    Search {
        /// Case-insensitive substring of the owner or repo name.
        query: Option<String>,
        #[arg(long, default_value_t = 30)]
        limit: u32,
    },
    /// Show a repository's details.
    View {
        /// OWNER/NAME. Defaults to the repo in the current directory.
        repo: Option<String>,
    },
    /// Create a new (empty) repository to push to.
    Create {
        name: String,
        #[arg(long)]
        private: bool,
        #[arg(long)]
        description: Option<String>,
        /// Skip the confirmation prompt.
        #[arg(short, long)]
        yes: bool,
    },
    /// Open a repository in your browser.
    Browse { repo: Option<String> },
}

#[derive(clap::Subcommand)]
pub enum PrCommand {
    /// List pull requests in a repository.
    List {
        repo: Option<String>,
        #[arg(long, default_value = "open")]
        state: String,
    },
    /// Show a pull request's details, comments, and diff.
    View { number: i64, repo: Option<String> },
    /// Print a pull request's unified diff.
    Diff { number: i64, repo: Option<String> },
    /// Open a pull request from one branch into another.
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        head: Option<String>,
        #[arg(long, default_value = "main")]
        base: String,
        #[arg(long)]
        body: Option<String>,
        repo: Option<String>,
    },
    /// Comment on a pull request.
    Comment {
        number: i64,
        #[arg(long)]
        body: String,
        repo: Option<String>,
    },
}

#[derive(clap::Subcommand)]
pub enum IssueCommand {
    /// List issues in a repository.
    List {
        repo: Option<String>,
        #[arg(long, default_value = "open")]
        state: String,
        #[arg(long)]
        query: Option<String>,
    },
    /// Show an issue's details and comments.
    View { number: i64, repo: Option<String> },
    /// Open a new issue.
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: Option<String>,
        repo: Option<String>,
    },
    /// Comment on an issue.
    Comment {
        number: i64,
        #[arg(long)]
        body: String,
        repo: Option<String>,
    },
}

#[derive(clap::Subcommand)]
pub enum CodeCommand {
    /// Print a file's contents at a ref.
    View {
        path: String,
        #[arg(long)]
        r#ref: Option<String>,
        repo: Option<String>,
    },
    /// Search a repository's tracked files for a fixed string.
    Search {
        query: String,
        #[arg(long)]
        r#ref: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        repo: Option<String>,
    },
}

#[derive(clap::Subcommand)]
pub enum HookCommand {
    /// List a repository's webhooks.
    List { repo: Option<String> },
    /// Register a new webhook. Prints the signing secret once — it cannot be
    /// retrieved again.
    Create {
        #[arg(long)]
        url: String,
        #[arg(long, value_delimiter = ',', default_value = "push")]
        events: Vec<String>,
        repo: Option<String>,
    },
    /// Remove a webhook.
    Delete {
        id: i64,
        repo: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(clap::Subcommand)]
pub enum SessionCommand {
    /// List your Cloud Sessions.
    List,
    /// Show a session and its transcript.
    View { id: i64 },
    /// Start a new session.
    New {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        model: Option<String>,
    },
    /// Rename a session or change its model.
    Update {
        id: i64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        model: Option<String>,
    },
    /// Delete a session.
    Delete {
        id: i64,
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(clap::Subcommand)]
pub enum BillingCommand {
    /// Show your current plan and cloud usage.
    Show,
    /// Open the hosted checkout to subscribe or change plans.
    Checkout { plan: String },
    /// Open the hosted billing portal.
    Portal,
}

#[derive(clap::Args)]
pub struct BrowseArgs {
    /// What to open: a repo, "OWNER/NAME/pull/N", "OWNER/NAME/issue/N", or
    /// left blank for the current repository.
    pub target: Option<String>,
}

#[derive(clap::Args)]
pub struct McpArgs {
    /// Drop every tool that writes, leaving only the ones that read.
    #[arg(long)]
    pub read_only: bool,

    /// Expose only these tool groups, comma-separated. Defaults to all of
    /// them: account, repo, issue, pr, code, hook, session, billing.
    #[arg(long, value_delimiter = ',', env = "SIGIT_TOOLSETS")]
    pub toolsets: Vec<String>,
}

#[derive(clap::Args)]
pub struct ApiArgs {
    /// Path under /api/v1, e.g. "repos" or "billing".
    pub path: String,
    #[arg(short = 'X', long, default_value = "GET")]
    pub method: String,
    /// Raw JSON request body.
    #[arg(long)]
    pub body: Option<String>,
}
