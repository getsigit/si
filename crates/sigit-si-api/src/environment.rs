//! Which sigit.si deployment a client talks to, and where its local state lives.

use std::{fmt, str::FromStr};

/// The deployment a [`crate::Client`] points at.
///
/// `SIGIT_HOST` overrides the host for either variant, so a client can be aimed
/// at a staging box or a tunnel without a code change:
///
/// ```text
/// SIGIT_HOST=sigit.example.com si repo list
/// SIGIT_HOST=http://localhost:4000 si repo list
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Environment {
    /// A local Rails server (`bin/rails s`), http on port 3000.
    Dev,
    /// https://sigit.si
    #[default]
    Production,
}

impl Environment {
    /// Scheme + host, no trailing slash — e.g. `https://sigit.si`.
    ///
    /// Both the JSON API and the web UI live on this origin, so it doubles as
    /// the base for browser URLs and `.git` clone URLs.
    pub fn base_url(&self) -> String {
        match std::env::var("SIGIT_HOST") {
            Ok(host) => normalize_host(host.trim(), self.default_scheme()),
            Err(_) => match self {
                Environment::Dev => "http://localhost:3000".to_string(),
                Environment::Production => "https://sigit.si".to_string(),
            },
        }
    }

    /// Base for JSON API calls — e.g. `https://sigit.si/api/v1`.
    pub fn api_url(&self) -> String {
        format!("{}/api/v1", self.base_url())
    }

    /// Directory under `$HOME` holding the token and config for this
    /// environment. Dev gets its own so a local login can't clobber the real
    /// one.
    pub fn config_dir_name(&self) -> &'static str {
        match self {
            Environment::Dev => ".sigit-dev",
            Environment::Production => ".sigit",
        }
    }

    fn default_scheme(&self) -> &'static str {
        match self {
            Environment::Dev => "http",
            Environment::Production => "https",
        }
    }
}

/// Accept `SIGIT_HOST` with or without a scheme, and strip any trailing slash.
fn normalize_host(host: &str, default_scheme: &str) -> String {
    let with_scheme = if host.contains("://") {
        host.to_string()
    } else {
        format!("{default_scheme}://{host}")
    };
    with_scheme.trim_end_matches('/').to_string()
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Environment::Dev => "dev",
            Environment::Production => "production",
        };
        f.write_str(name)
    }
}

impl FromStr for Environment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "dev" | "development" | "local" => Ok(Environment::Dev),
            "production" | "prod" => Ok(Environment::Production),
            other => Err(format!(
                "Unknown environment {other:?}. Valid values are: dev, production."
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_url_is_the_base_plus_the_version_prefix() {
        // SIGIT_HOST is process-global; this asserts the un-overridden default,
        // which is what every test process sees unless one sets the var.
        if std::env::var("SIGIT_HOST").is_ok() {
            return;
        }
        assert_eq!(Environment::Production.api_url(), "https://sigit.si/api/v1");
        assert_eq!(Environment::Dev.api_url(), "http://localhost:3000/api/v1");
    }

    #[test]
    fn bare_hosts_get_the_environment_default_scheme() {
        assert_eq!(
            normalize_host("sigit.example.com", "https"),
            "https://sigit.example.com"
        );
        assert_eq!(
            normalize_host("localhost:4000", "http"),
            "http://localhost:4000"
        );
    }

    #[test]
    fn explicit_schemes_and_trailing_slashes_survive_normalization() {
        assert_eq!(
            normalize_host("http://localhost:4000/", "https"),
            "http://localhost:4000"
        );
    }

    #[test]
    fn environments_round_trip_through_strings() {
        assert_eq!("dev".parse(), Ok(Environment::Dev));
        assert_eq!("PROD".parse(), Ok(Environment::Production));
        assert!("staging".parse::<Environment>().is_err());
    }
}
