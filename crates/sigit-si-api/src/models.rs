//! Wire types for the sigit.si API.
//!
//! Field names and nullability mirror the Rails serializers in
//! `app/controllers/api/v1/` and the MCP tool results in
//! `app/services/mcp/tools.rb`. Every struct is `deny_unknown_fields`-free on
//! purpose: the server may add fields, and an older `si` should keep working.

use {
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
};

// ── Account ─────────────────────────────────────────────────────────────────

/// `GET /api/v1/user`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

/// The result of `POST /api/v1/auth/sign_in`, shaped as an
/// externally tagged enum: the bare string `"NotFound"`, `{"Ready":{…}}`, or
/// `{"Incomplete":{…}}`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum AccountStatus {
    NotFound,
    Ready { access_token: String },
    Incomplete { status: u32 },
}

// ── Repositories ────────────────────────────────────────────────────────────

/// `GET /api/v1/repos`, `POST /api/v1/repos`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Repository {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub default_branch: String,
    pub kind: Option<String>,
    pub is_private: bool,
    pub stars_count: i64,
    pub initialized: bool,
    pub updated_at: DateTime<Utc>,
    pub web_url: String,
    pub clone_url: String,
}

impl Repository {
    pub fn visibility(&self) -> &'static str {
        if self.is_private {
            "private"
        } else {
            "public"
        }
    }
}

// ── Webhooks ────────────────────────────────────────────────────────────────

/// `GET/POST /api/v1/repos/:owner/:repo/hooks`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Hook {
    pub id: i64,
    pub url: String,
    pub events: Vec<String>,
    pub active: bool,
    pub last_status_code: Option<i32>,
    pub last_delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    /// Returned exactly once, by `POST`. It is not retrievable afterwards.
    #[serde(default)]
    pub secret: Option<String>,
}

// ── Cloud Sessions ──────────────────────────────────────────────────────────

/// `GET /api/v1/sessions`, `GET /api/v1/sessions/:id`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CloudSession {
    pub id: i64,
    pub title: Option<String>,
    pub model: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Only present on the single-session `show` response.
    #[serde(default)]
    pub messages: Option<Vec<CloudMessage>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CloudMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// `POST /api/v1/sessions/:id/messages`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppendedMessage {
    pub session: CloudSession,
    pub message: CloudMessage,
}

// ── Billing ─────────────────────────────────────────────────────────────────

/// `GET /api/v1/billing`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Billing {
    pub plan: String,
    pub status: String,
    pub entitled_to_cloud: bool,
    pub cloud_requests_used: i64,
    pub cloud_allowance: i64,
    pub current_period_end: Option<DateTime<Utc>>,
}

/// `POST /api/v1/billing/checkout`, `POST /api/v1/billing/portal`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BillingUrl {
    pub url: String,
}

// ── Git credentials ─────────────────────────────────────────────────────────

/// `POST /api/v1/git_credentials` — a short-lived, read-scoped git password.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitCredential {
    pub git_token: String,
    pub username: String,
    pub expires_at: DateTime<Utc>,
}

// ── Commit statuses ─────────────────────────────────────────────────────────

/// One reported status on a commit, from `POST …/statuses/:sha` and the
/// `statuses` list of the combined status.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitStatus {
    pub id: i64,
    pub sha: String,
    /// `pending`, `success`, `failure` or `error`.
    pub state: String,
    pub context: String,
    pub description: Option<String>,
    pub target_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// `GET …/commits/:sha/status` — every context on a commit and the state they
/// add up to.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CombinedStatus {
    pub sha: String,
    pub state: String,
    pub total_count: u32,
    pub statuses: Vec<CommitStatus>,
}

// ── Agent runs ──────────────────────────────────────────────────────────────

/// A siGit Code Cloud Agent run. `transcript_tail` is only present on the
/// single-run read.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentRun {
    pub id: i64,
    pub title: Option<String>,
    pub repo_full_name: String,
    pub trigger_kind: String,
    pub status: String,
    #[serde(default)]
    pub cancel_requested: bool,
    pub exit_reason: Option<String>,
    pub base_branch: Option<String>,
    pub head_branch: Option<String>,
    pub pull_request_url: Option<String>,
    pub issue_number: Option<i64>,
    pub pr_number: Option<i64>,
    #[serde(default)]
    pub inference_requests_used: i64,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript_tail: Option<String>,
}

// ── Sites ───────────────────────────────────────────────────────────────────

/// A repository's published static site.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Site {
    pub url: String,
    /// `publishing`, `live` or `failed`.
    pub state: String,
    pub error: Option<String>,
    pub last_published_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub files: i64,
    #[serde(default)]
    pub bytes: i64,
}

// ── Models ──────────────────────────────────────────────────────────────────

/// One entry of `GET /api/v1/models`, the models you can pick for a session.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Model {
    pub id: String,
    pub display_name: Option<String>,
    pub capability: Option<String>,
    #[serde(default)]
    pub vision: bool,
    /// Set on the model the calling client uses when none is named.
    #[serde(default)]
    pub default: bool,
}

// ── MCP-backed resources ────────────────────────────────────────────────────
//
// Issues, pull requests, and code search have no REST surface; they are served
// by the MCP tools at `POST /api/v1/mcp`. These are the tool result shapes.

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepositorySummary {
    pub full_name: String,
    pub description: Option<String>,
    pub default_branch: String,
    pub private: bool,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileContents {
    pub repo: String,
    pub path: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub size: i64,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeHit {
    pub path: String,
    pub line: i64,
    pub snippet: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Comment {
    pub author: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IssueSummary {
    pub number: i64,
    pub title: String,
    pub state: String,
    pub author: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Issue {
    pub number: i64,
    pub title: String,
    pub state: String,
    pub author: String,
    pub body: Option<String>,
    pub url: String,
    #[serde(default)]
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullRequestSummary {
    pub number: i64,
    pub title: String,
    pub state: String,
    pub head: String,
    pub base: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullRequest {
    pub number: i64,
    pub title: String,
    pub state: String,
    pub author: String,
    pub head: String,
    pub base: String,
    pub body: Option<String>,
    pub url: String,
    #[serde(default)]
    pub comments: Vec<Comment>,
    /// `None` when a branch no longer exists; see `diff_note`.
    #[serde(default)]
    pub diff: Option<String>,
    #[serde(default)]
    pub diff_truncated: bool,
    #[serde(default)]
    pub diff_note: Option<String>,
}

/// What `create_issue` / `create_pull_request` hand back.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreatedRef {
    pub number: i64,
    pub state: String,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_status_reads_the_bare_not_found_string() {
        let status: AccountStatus = serde_json::from_str(r#""NotFound""#).unwrap();
        assert!(matches!(status, AccountStatus::NotFound));
    }

    #[test]
    fn account_status_reads_the_ready_envelope() {
        let status: AccountStatus =
            serde_json::from_str(r#"{"Ready":{"access_token":"abc123"}}"#).unwrap();
        match status {
            AccountStatus::Ready { access_token } => assert_eq!(access_token, "abc123"),
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn account_status_reads_the_incomplete_envelope() {
        let status: AccountStatus =
            serde_json::from_str(r#"{"Incomplete":{"status":1001}}"#).unwrap();
        match status {
            AccountStatus::Incomplete { status } => assert_eq!(status, 1001),
            other => panic!("expected Incomplete, got {other:?}"),
        }
    }

    #[test]
    fn a_hook_without_a_secret_still_parses() {
        // Only the create response carries `secret`; index responses omit it.
        let hook: Hook = serde_json::from_str(
            r#"{"id":1,"url":"https://x.test","events":["push"],"active":true,
                "last_status_code":null,"last_delivered_at":null,
                "created_at":"2026-08-16T10:00:00.000Z"}"#,
        )
        .unwrap();
        assert!(hook.secret.is_none());
        assert!(hook.last_delivered_at.is_none());
    }

    #[test]
    fn file_contents_maps_the_ref_keyword_field() {
        let file: FileContents = serde_json::from_str(
            r#"{"repo":"a/b","path":"README.md","ref":"main","size":3,"content":"hi\n"}"#,
        )
        .unwrap();
        assert_eq!(file.git_ref, "main");
    }

    #[test]
    fn a_pull_request_without_diff_fields_parses() {
        // `get_pull_request` omits diff_truncated unless it truncated.
        let pr: PullRequest = serde_json::from_str(
            r#"{"number":7,"title":"t","state":"open","author":"seto","head":"f","base":"main",
                "body":null,"url":"https://sigit.si/a/b/pull/7","comments":[],"diff":"@@"}"#,
        )
        .unwrap();
        assert!(!pr.diff_truncated);
        assert_eq!(pr.diff.as_deref(), Some("@@"));
    }
}
