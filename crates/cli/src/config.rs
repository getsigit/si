//! Where `si` keeps its credentials.
//!
//! One file per environment under `$HOME`: `~/.sigit/token` for production,
//! `~/.sigit-dev/token` for a local server. `SIGIT_TOKEN` overrides both, which
//! is how CI authenticates without a login step.

use {
    anyhow::{anyhow, Context, Result},
    sigit_api::Environment,
    std::{
        fs,
        path::{Path, PathBuf},
    },
};

/// Environment variable holding a token, checked before the token file.
pub const TOKEN_ENV: &str = "SIGIT_TOKEN";

/// `~/.sigit` (or `~/.sigit-dev`), created if absent.
pub fn config_dir(environment: Environment) -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not locate your home directory."))?;
    let dir = home.join(environment.config_dir_name());
    fs::create_dir_all(&dir).with_context(|| format!("Could not create {}.", dir.display()))?;
    Ok(dir)
}

pub fn token_path(environment: Environment) -> Result<PathBuf> {
    Ok(config_dir(environment)?.join("token"))
}

/// The token for this environment, from `SIGIT_TOKEN` or the token file.
///
/// Returns `None` rather than an error when there is simply no token — "not
/// signed in" is an expected state that callers phrase themselves.
pub fn read_token(environment: Environment) -> Option<String> {
    if let Ok(token) = std::env::var(TOKEN_ENV) {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }

    let path = token_path(environment).ok()?;
    let token = fs::read_to_string(path).ok()?.trim().to_string();
    (!token.is_empty()).then_some(token)
}

/// Persist a token, readable only by its owner.
pub fn write_token(environment: Environment, token: &str) -> Result<PathBuf> {
    let path = token_path(environment)?;
    fs::write(&path, token.trim())
        .with_context(|| format!("Could not write {}.", path.display()))?;
    restrict_permissions(&path)?;
    Ok(path)
}

/// Remove the stored token. Succeeds when there was nothing to remove — sign
/// out should be idempotent, including after a token has already expired.
pub fn clear_token(environment: Environment) -> Result<()> {
    let path = token_path(environment)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("Could not remove {}.", path.display())),
    }
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("Could not set permissions on {}.", path.display()))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<()> {
    // Windows inherits the user-profile ACL, which is already owner-only.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_environment_gets_its_own_directory() {
        assert_ne!(
            Environment::Dev.config_dir_name(),
            Environment::Production.config_dir_name()
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_written_token_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token");
        fs::write(&path, "secret").unwrap();
        restrict_permissions(&path).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
