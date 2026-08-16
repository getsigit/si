//! Working-directory git awareness.
//!
//! The point of a `gh`-style CLI is that you rarely have to name the repo — it
//! reads the one you're standing in. This module answers two questions about
//! the current directory: which sigit.si repo does it point at, and what branch
//! is checked out.

use {
    anyhow::{anyhow, Context, Result},
    git2::Repository as GitRepository,
    std::{fmt, path::Path},
};

/// An `owner/name` pair identifying a repository on sigit.si.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSlug {
    pub owner: String,
    pub name: String,
}

impl RepoSlug {
    /// Parse the `owner/name` form users type into `--repo`.
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim().trim_matches('/');
        let mut parts = trimmed.splitn(2, '/');
        let owner = parts.next().unwrap_or_default().trim();
        let name = parts.next().unwrap_or_default().trim();

        if owner.is_empty() || name.is_empty() || name.contains('/') {
            return Err(anyhow!(
                "Expected a repository in OWNER/NAME form, got {input:?}."
            ));
        }

        Ok(Self {
            owner: owner.to_string(),
            name: name.trim_end_matches(".git").to_string(),
        })
    }
}

impl fmt::Display for RepoSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// Find the sigit.si repository the given directory belongs to.
///
/// Remotes are searched in preference order — `origin`, then `sigit`, then
/// whatever else is configured — and only remotes whose host matches `host` are
/// considered, so a repo with both a GitHub and a sigit.si remote resolves to
/// the sigit.si one.
pub fn current_repo(dir: &Path, host: &str) -> Result<RepoSlug> {
    let repository = GitRepository::discover(dir)
        .context("Not inside a git repository. Pass --repo OWNER/NAME instead.")?;

    let remotes = repository
        .remotes()
        .context("Could not read this repository's remotes.")?;

    let names: Vec<String> = remotes.iter().flatten().map(str::to_string).collect();

    for candidate in preferred_order(&names) {
        let Ok(remote) = repository.find_remote(&candidate) else {
            continue;
        };
        let Some(url) = remote.url() else { continue };
        if let Some(slug) = slug_from_url(url, host) {
            return Ok(slug);
        }
    }

    Err(anyhow!(
        "No {host} remote found in this repository. Pass --repo OWNER/NAME instead."
    ))
}

/// The branch currently checked out, or `None` on a detached HEAD.
pub fn current_branch(dir: &Path) -> Option<String> {
    let repository = GitRepository::discover(dir).ok()?;
    let head = repository.head().ok()?;
    if !head.is_branch() {
        return None;
    }
    head.shorthand().map(str::to_string)
}

/// `origin` first, then `sigit`, then the rest in their configured order.
fn preferred_order(names: &[String]) -> Vec<String> {
    let mut ordered = Vec::with_capacity(names.len());
    for preferred in ["origin", "sigit"] {
        if names.iter().any(|n| n == preferred) {
            ordered.push(preferred.to_string());
        }
    }
    for name in names {
        if !ordered.contains(name) {
            ordered.push(name.clone());
        }
    }
    ordered
}

/// Pull `owner/name` out of a remote URL, but only when it points at `host`.
///
/// Handles the three shapes git remotes come in: `https://host/owner/repo.git`,
/// `git@host:owner/repo.git`, and `ssh://git@host/owner/repo.git`.
fn slug_from_url(url: &str, host: &str) -> Option<RepoSlug> {
    let url = url.trim();
    let (url_host, path) = split_host_and_path(url)?;

    if !hosts_match(&url_host, host) {
        return None;
    }

    RepoSlug::parse(path.trim_matches('/')).ok()
}

fn split_host_and_path(url: &str) -> Option<(String, String)> {
    if let Some(rest) = url.split_once("://").map(|(_, rest)| rest) {
        // scheme://[user@]host[:port]/path
        let rest = rest.split_once('@').map_or(rest, |(_, after)| after);
        let (authority, path) = rest.split_once('/')?;
        return Some((authority.to_string(), path.to_string()));
    }

    // scp-like: [user@]host:path
    let rest = url.split_once('@').map_or(url, |(_, after)| after);
    let (authority, path) = rest.split_once(':')?;
    Some((authority.to_string(), path.to_string()))
}

/// Compare hosts ignoring the port and a leading `www.`, so a dev remote on
/// `localhost:3000` matches a `localhost` base URL and vice versa.
fn hosts_match(a: &str, b: &str) -> bool {
    fn normalize(host: &str) -> String {
        let host = host.trim().to_ascii_lowercase();
        let host = host
            .rsplit_once("://")
            .map_or(host.as_str(), |(_, rest)| rest);
        let host = host.split('/').next().unwrap_or(host);
        let host = host.split(':').next().unwrap_or(host);
        host.trim_start_matches("www.").to_string()
    }

    normalize(a) == normalize(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_parse_from_the_owner_name_form() {
        let slug = RepoSlug::parse("seto/sigit-si").unwrap();
        assert_eq!(slug.owner, "seto");
        assert_eq!(slug.name, "sigit-si");
        assert_eq!(slug.to_string(), "seto/sigit-si");
    }

    #[test]
    fn a_trailing_git_suffix_is_dropped() {
        assert_eq!(
            RepoSlug::parse("seto/sigit-si.git").unwrap().name,
            "sigit-si"
        );
    }

    #[test]
    fn dotted_repo_names_survive() {
        // The server allows names like "Qwen2.5-3B-Instruct-GGUF".
        let slug = RepoSlug::parse("seto/Qwen2.5-3B-Instruct-GGUF").unwrap();
        assert_eq!(slug.name, "Qwen2.5-3B-Instruct-GGUF");
    }

    #[test]
    fn incomplete_slugs_are_rejected() {
        assert!(RepoSlug::parse("sigit-si").is_err());
        assert!(RepoSlug::parse("seto/").is_err());
        assert!(RepoSlug::parse("/sigit-si").is_err());
        assert!(RepoSlug::parse("a/b/c").is_err());
    }

    #[test]
    fn https_remotes_resolve() {
        let slug = slug_from_url("https://sigit.si/seto/sigit-si.git", "https://sigit.si").unwrap();
        assert_eq!(slug.to_string(), "seto/sigit-si");
    }

    #[test]
    fn scp_style_ssh_remotes_resolve() {
        let slug = slug_from_url("git@sigit.si:seto/sigit-si.git", "sigit.si").unwrap();
        assert_eq!(slug.to_string(), "seto/sigit-si");
    }

    #[test]
    fn ssh_url_remotes_resolve() {
        let slug = slug_from_url("ssh://git@sigit.si/seto/sigit-si.git", "sigit.si").unwrap();
        assert_eq!(slug.to_string(), "seto/sigit-si");
    }

    #[test]
    fn remotes_on_another_host_are_ignored() {
        // The whole point: a repo mirrored from GitHub must not resolve to its
        // GitHub remote when we asked for the sigit.si one.
        assert!(slug_from_url("git@github.com:seto/sigit-si.git", "sigit.si").is_none());
        assert!(
            slug_from_url("https://github.com/seto/sigit-si.git", "https://sigit.si").is_none()
        );
    }

    #[test]
    fn dev_remotes_match_regardless_of_port() {
        let slug = slug_from_url(
            "http://localhost:3000/seto/demo.git",
            "http://localhost:3000",
        )
        .unwrap();
        assert_eq!(slug.to_string(), "seto/demo");
    }

    #[test]
    fn origin_is_tried_before_other_remotes() {
        let names = vec!["upstream".to_string(), "origin".to_string()];
        assert_eq!(preferred_order(&names).first().unwrap(), "origin");
    }

    #[test]
    fn every_remote_is_still_considered() {
        let names = vec!["upstream".to_string(), "origin".to_string()];
        assert_eq!(preferred_order(&names).len(), 2);
    }
}
