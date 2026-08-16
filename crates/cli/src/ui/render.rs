//! Human-readable renderers. Each is the `--json` counterpart of one command's
//! output; see [`crate::ui::emit`].

use {
    super::{bold, dim, highlight, relative_time, state_badge},
    sigit_si_api::models::{
        Billing, CloudSession, CodeHit, Comment, Hook, Issue, IssueSummary, PullRequest,
        PullRequestSummary, Repository, RepositorySummary, User,
    },
    tabled::{builder::Builder, settings::Style},
};

/// Print a table, or a placeholder line when there is nothing to show.
fn table(headers: &[&str], rows: Vec<Vec<String>>, empty: &str) {
    if rows.is_empty() {
        println!("{}", dim(empty));
        return;
    }

    let mut builder = Builder::default();
    builder.push_record(headers.iter().map(|h| h.to_string()));
    for row in rows {
        builder.push_record(row);
    }

    let mut table = builder.build();
    table.with(Style::rounded());
    println!("{table}");
}

/// A `label  value` line, skipped when the value is empty.
fn field(label: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) {
        println!("  {:<14}{}", label, value);
    }
}

fn truncate(text: &str, max: usize) -> String {
    let text = text.replace(['\n', '\r'], " ");
    if text.chars().count() <= max {
        return text;
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

// ── Account ─────────────────────────────────────────────────────────────────

pub fn user(user: &User) {
    println!();
    println!("  {}", highlight(&user.username));
    field("Name", user.display_name.as_deref());
    field("Email", Some(&user.email));
    field("ID", Some(&user.id.to_string()));
    field(
        "Joined",
        Some(&user.created_at.format("%Y-%m-%d").to_string()),
    );
    println!();
}

// ── Repositories ────────────────────────────────────────────────────────────

pub fn repos(repos: &[Repository]) {
    let rows = repos
        .iter()
        .map(|repo| {
            vec![
                repo.full_name.clone(),
                repo.visibility().to_string(),
                truncate(repo.description.as_deref().unwrap_or("—"), 48),
                repo.stars_count.to_string(),
                relative_time(repo.updated_at),
            ]
        })
        .collect();

    table(
        &[
            "Repository",
            "Visibility",
            "Description",
            "Stars",
            "Updated",
        ],
        rows,
        "You have no repositories yet. Create one with `si repo create <name>`.",
    );
}

pub fn repo_summaries(repos: &[RepositorySummary]) {
    let rows = repos
        .iter()
        .map(|repo| {
            vec![
                repo.full_name.clone(),
                if repo.private { "private" } else { "public" }.to_string(),
                truncate(repo.description.as_deref().unwrap_or("—"), 56),
                repo.default_branch.clone(),
            ]
        })
        .collect();

    table(
        &["Repository", "Visibility", "Description", "Branch"],
        rows,
        "No repositories matched.",
    );
}

pub fn repo_detail(repo: &Repository) {
    println!();
    println!(
        "  {}  {}",
        highlight(&repo.full_name),
        dim(repo.visibility())
    );
    if let Some(description) = repo.description.as_deref().filter(|d| !d.is_empty()) {
        println!("  {description}");
    }
    println!();
    field("Default branch", Some(&repo.default_branch));
    field("Stars", Some(&repo.stars_count.to_string()));
    field(
        "State",
        Some(if repo.initialized {
            "initialized"
        } else {
            "empty — push something to get started"
        }),
    );
    field("Updated", Some(&relative_time(repo.updated_at)));
    field("Clone", Some(&repo.clone_url));
    field("Web", Some(&repo.web_url));
    println!();
}

// ── Pull requests ───────────────────────────────────────────────────────────

pub fn pulls(pulls: &[PullRequestSummary]) {
    let rows = pulls
        .iter()
        .map(|pr| {
            vec![
                format!("#{}", pr.number),
                truncate(&pr.title, 52),
                state_badge(&pr.state),
                format!("{} → {}", pr.head, pr.base),
            ]
        })
        .collect();

    table(
        &["#", "Title", "State", "Branch"],
        rows,
        "No pull requests match this filter.",
    );
}

pub fn pull_detail(pr: &PullRequest) {
    println!();
    println!("  {}", bold(&pr.title));
    println!(
        "  {} · {} · {} wants to merge {} into {}",
        dim(&format!("#{}", pr.number)),
        state_badge(&pr.state),
        pr.author,
        highlight(&pr.head),
        highlight(&pr.base),
    );
    println!();

    if let Some(body) = pr.body.as_deref().filter(|b| !b.trim().is_empty()) {
        println!("{body}");
        println!();
    }

    comments(&pr.comments);

    if let Some(note) = &pr.diff_note {
        println!("  {}", dim(note));
    }
    let changed = pr.diff.as_deref().map_or(0, count_changed_files);
    if changed > 0 {
        println!(
            "  {}",
            dim(&format!(
                "{changed} changed file(s) — see them with `si pr diff {}`",
                pr.number
            ))
        );
    }
    println!("  {}", dim(&pr.url));
    println!();
}

/// Count `diff --git` headers, which is one per changed file.
fn count_changed_files(diff: &str) -> usize {
    diff.lines()
        .filter(|l| l.starts_with("diff --git "))
        .count()
}

// ── Issues ──────────────────────────────────────────────────────────────────

pub fn issues(issues: &[IssueSummary]) {
    let rows = issues
        .iter()
        .map(|issue| {
            vec![
                format!("#{}", issue.number),
                truncate(&issue.title, 60),
                state_badge(&issue.state),
                issue.author.clone(),
            ]
        })
        .collect();

    table(
        &["#", "Title", "State", "Author"],
        rows,
        "No issues match this filter.",
    );
}

pub fn issue_detail(issue: &Issue) {
    println!();
    println!("  {}", bold(&issue.title));
    println!(
        "  {} · {} · opened by {}",
        dim(&format!("#{}", issue.number)),
        state_badge(&issue.state),
        issue.author
    );
    println!();

    if let Some(body) = issue.body.as_deref().filter(|b| !b.trim().is_empty()) {
        println!("{body}");
        println!();
    }

    comments(&issue.comments);
    println!("  {}", dim(&issue.url));
    println!();
}

pub fn comments(comments: &[Comment]) {
    for comment in comments {
        println!(
            "  {} {}",
            highlight(&comment.author),
            dim(&relative_time(comment.created_at))
        );
        for line in comment.body.lines() {
            println!("    {line}");
        }
        println!();
    }
}

// ── Code search ─────────────────────────────────────────────────────────────

pub fn code_hits(hits: &[CodeHit]) {
    if hits.is_empty() {
        println!("{}", dim("No matches."));
        return;
    }

    for hit in hits {
        println!(
            "{}{}{}",
            highlight(&hit.path),
            dim(":"),
            dim(&hit.line.to_string())
        );
        println!("  {}", hit.snippet.trim_end());
    }
}

// ── Webhooks ────────────────────────────────────────────────────────────────

pub fn hooks(hooks: &[Hook]) {
    let rows = hooks
        .iter()
        .map(|hook| {
            vec![
                hook.id.to_string(),
                truncate(&hook.url, 52),
                hook.events.join(", "),
                if hook.active { "active" } else { "paused" }.to_string(),
                hook.last_delivered_at
                    .map(relative_time)
                    .unwrap_or_else(|| "never".to_string()),
            ]
        })
        .collect();

    table(
        &["ID", "URL", "Events", "State", "Last delivery"],
        rows,
        "This repository has no webhooks.",
    );
}

// ── Cloud Sessions ──────────────────────────────────────────────────────────

pub fn sessions(sessions: &[CloudSession]) {
    let rows = sessions
        .iter()
        .map(|session| {
            vec![
                session.id.to_string(),
                truncate(session.title.as_deref().unwrap_or("(untitled)"), 48),
                session.model.clone().unwrap_or_else(|| "—".to_string()),
                session
                    .last_message_at
                    .map(relative_time)
                    .unwrap_or_else(|| "—".to_string()),
            ]
        })
        .collect();

    table(
        &["ID", "Title", "Model", "Last message"],
        rows,
        "You have no cloud sessions yet. Start one with `si session new`.",
    );
}

pub fn session_detail(session: &CloudSession) {
    println!();
    println!(
        "  {}",
        highlight(session.title.as_deref().unwrap_or("(untitled session)"))
    );
    field("ID", Some(&session.id.to_string()));
    field("Model", session.model.as_deref());
    field("Created", Some(&relative_time(session.created_at)));
    println!();

    let Some(messages) = &session.messages else {
        return;
    };
    if messages.is_empty() {
        println!("{}", dim("  No messages yet."));
        println!();
        return;
    }

    for message in messages {
        println!(
            "  {} {}",
            highlight(&message.role),
            dim(&relative_time(message.created_at))
        );
        for line in message.content.lines() {
            println!("    {line}");
        }
        println!();
    }
}

// ── Billing ─────────────────────────────────────────────────────────────────

pub fn billing(billing: &Billing) {
    println!();
    println!("  {}  {}", highlight(&billing.plan), dim(&billing.status));
    field(
        "Cloud",
        Some(if billing.entitled_to_cloud {
            "entitled"
        } else {
            "not included on this plan"
        }),
    );
    field(
        "Usage",
        Some(&format!(
            "{} / {} requests",
            billing.cloud_requests_used, billing.cloud_allowance
        )),
    );
    if let Some(period_end) = billing.current_period_end {
        field("Renews", Some(&period_end.format("%Y-%m-%d").to_string()));
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_text_is_truncated_with_an_ellipsis() {
        assert_eq!(truncate("abcdefghij", 5), "abcd…");
        assert_eq!(truncate("abc", 5), "abc");
    }

    #[test]
    fn newlines_are_flattened_so_a_row_stays_one_line() {
        assert_eq!(truncate("a\nb", 10), "a b");
    }

    #[test]
    fn changed_files_are_counted_from_diff_headers() {
        let diff = "diff --git a/x b/x\n@@ -1 +1 @@\n-a\n+b\ndiff --git a/y b/y\n";
        assert_eq!(count_changed_files(diff), 2);
        assert_eq!(count_changed_files(""), 0);
    }
}
