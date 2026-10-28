//! Git's credential helper protocol, and the `git` command lines built around
//! it.
//!
//! Git asks a helper for a password by writing `key=value` lines to its stdin,
//! ended by a blank line, and reads `key=value` lines back. `si` answers for
//! the sigit.si host only, with a short-lived token minted on demand, so no
//! credential is ever stored.

use {
    crate::git::{hosts_match, RepoSlug},
    anyhow::{anyhow, Context, Result},
    std::{collections::HashMap, io::BufRead, path::Path, process::Command},
};

/// The attributes of one credential request, e.g. `protocol`, `host`, `path`.
pub type Request = HashMap<String, String>;

/// Read a request up to the first blank line, or the end of input.
pub fn parse_request(input: impl BufRead) -> Request {
    let mut request = Request::new();
    for line in input.lines().map_while(|line| line.ok()) {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once('=') {
            request.insert(key.to_string(), value.to_string());
        }
    }
    request
}

/// Whether the request is for `base_url`, such as `https://sigit.si`. A
/// request for another host is none of our business, and a request over a
/// different scheme must not get a token that was meant for the secure one.
pub fn is_for_host(request: &Request, base_url: &str) -> bool {
    let Some(host) = request.get("host") else {
        return false;
    };
    if !hosts_match(host, base_url) {
        return false;
    }
    match (request.get("protocol"), base_url.split_once("://")) {
        (Some(protocol), Some((scheme, _))) => protocol.eq_ignore_ascii_case(scheme),
        _ => true,
    }
}

/// The reply git expects: two attributes and a blank line.
pub fn response(username: &str, password: &str) -> String {
    format!("username={username}\npassword={password}\n\n")
}

/// `credential.<base>.helper`, the config key that scopes a helper to one
/// host.
pub fn helper_key(base_url: &str) -> String {
    format!("credential.{base_url}.helper")
}

/// The helper command git runs, as a shell snippet: git appends the operation.
pub fn helper_command(exe: &Path) -> String {
    let exe = exe.to_string_lossy().replace('\'', r"'\''");
    format!("!'{exe}' auth git-credential")
}

/// The arguments for `git clone`. The leading empty `helper` entry clears any
/// helper inherited from the user's config, so the one that follows is the only
/// one asked.
pub fn clone_args(
    base_url: &str,
    exe: &Path,
    slug: &RepoSlug,
    dir: Option<&str>,
    extra: &[String],
) -> Vec<String> {
    let key = helper_key(base_url);
    let mut args = vec![
        "-c".to_string(),
        format!("{key}="),
        "-c".to_string(),
        format!("{key}={}", helper_command(exe)),
        "clone".to_string(),
    ];
    args.extend(extra.iter().cloned());
    args.push(format!("{base_url}/{}/{}.git", slug.owner, slug.name));
    args.extend(dir.map(str::to_string));
    args
}

fn git() -> Command {
    Command::new("git")
}

fn not_found(error: std::io::Error) -> anyhow::Error {
    if error.kind() == std::io::ErrorKind::NotFound {
        anyhow!("`git` was not found on your PATH. Install git and try again.")
    } else {
        anyhow!("Could not run git: {error}")
    }
}

/// Run `git clone` with the helper in place.
pub fn clone(args: &[String]) -> Result<()> {
    let status = git().args(args).status().map_err(not_found)?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("git clone failed ({status})."))
    }
}

/// Register the helper for `base_url` in the global git config, replacing any
/// earlier `si` entry. The empty entry first keeps a system-wide helper (such
/// as the macOS keychain) from answering for this host as well.
pub fn setup_global_helper(base_url: &str, exe: &Path) -> Result<()> {
    let key = helper_key(base_url);

    // Clear what we wrote before; exit code 5 only means there was none.
    git()
        .args(["config", "--global", "--unset-all", &key])
        .status()
        .map_err(not_found)?;

    for value in [String::new(), helper_command(exe)] {
        let status = git()
            .args(["config", "--global", "--add", &key, &value])
            .status()
            .map_err(not_found)
            .context("Could not write the global git config.")?;
        if !status.success() {
            return Err(anyhow!("`git config --global` failed ({status})."));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(input: &str) -> Request {
        parse_request(input.as_bytes())
    }

    #[test]
    fn a_request_is_read_up_to_the_blank_line() {
        let request = request("protocol=https\nhost=sigit.si\n\nignored=yes\n");
        assert_eq!(request["host"], "sigit.si");
        assert_eq!(request["protocol"], "https");
        assert!(!request.contains_key("ignored"));
    }

    #[test]
    fn a_request_may_end_without_a_blank_line() {
        assert_eq!(request("host=sigit.si")["host"], "sigit.si");
    }

    #[test]
    fn values_may_contain_equals_signs() {
        assert_eq!(request("path=a=b\n")["path"], "a=b");
    }

    #[test]
    fn the_configured_host_matches() {
        let request = request("protocol=https\nhost=sigit.si\n\n");
        assert!(is_for_host(&request, "https://sigit.si"));
    }

    #[test]
    fn other_hosts_are_ignored() {
        let request = request("protocol=https\nhost=github.com\n\n");
        assert!(!is_for_host(&request, "https://sigit.si"));
    }

    #[test]
    fn a_request_without_a_host_is_ignored() {
        assert!(!is_for_host(
            &request("protocol=https\n\n"),
            "https://sigit.si"
        ));
    }

    #[test]
    fn a_plain_http_request_does_not_get_a_token_for_an_https_host() {
        let request = request("protocol=http\nhost=sigit.si\n\n");
        assert!(!is_for_host(&request, "https://sigit.si"));
    }

    #[test]
    fn a_dev_host_matches_with_its_port() {
        let request = request("protocol=http\nhost=localhost:3000\n\n");
        assert!(is_for_host(&request, "http://localhost:3000"));
    }

    #[test]
    fn the_response_ends_with_a_blank_line() {
        assert_eq!(
            response("x-access-token", "abc"),
            "username=x-access-token\npassword=abc\n\n"
        );
    }

    #[test]
    fn the_helper_command_quotes_the_executable() {
        let command = helper_command(Path::new("/opt/it's here/si"));
        assert_eq!(command, r"!'/opt/it'\''s here/si' auth git-credential");
    }

    #[test]
    fn clone_resets_the_helper_before_setting_ours() {
        let slug = RepoSlug::parse("seto/demo").unwrap();
        let args = clone_args(
            "https://sigit.si",
            Path::new("/bin/si"),
            &slug,
            Some("out"),
            &["--depth".to_string(), "1".to_string()],
        );
        assert_eq!(
            args,
            [
                "-c",
                "credential.https://sigit.si.helper=",
                "-c",
                "credential.https://sigit.si.helper=!'/bin/si' auth git-credential",
                "clone",
                "--depth",
                "1",
                "https://sigit.si/seto/demo.git",
                "out",
            ]
        );
    }

    #[test]
    fn clone_without_a_directory_leaves_the_choice_to_git() {
        let slug = RepoSlug::parse("seto/demo").unwrap();
        let args = clone_args("https://sigit.si", Path::new("/bin/si"), &slug, None, &[]);
        assert_eq!(args.last().unwrap(), "https://sigit.si/seto/demo.git");
    }
}
