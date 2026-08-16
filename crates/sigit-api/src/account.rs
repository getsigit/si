//! Sign in, sign out, and the current-user profile.

use crate::{
    client::Client,
    error::Result,
    models::{AccountStatus, User},
};

impl Client {
    /// `POST /api/v1/auth/sign_in`
    ///
    /// Answers `200` for all three outcomes — the caller inspects the
    /// [`AccountStatus`] rather than the status code.
    pub async fn sign_in(&self, email: &str, password: &str) -> Result<AccountStatus> {
        let body = serde_json::json!({
            "email": email.trim().to_ascii_lowercase(),
            "password": password,
        });
        self.post("auth/sign_in", &body).await
    }

    /// `DELETE /api/v1/auth/sign_out` — revokes the token server-side.
    pub async fn sign_out(&self) -> Result<()> {
        self.delete("auth/sign_out").await
    }

    /// `GET /api/v1/user` — the signed-in user's profile.
    pub async fn user(&self) -> Result<User> {
        self.get("user").await
    }
}

/// Human-readable explanation for an `AccountStatus::Incomplete` status code,
/// mirroring `account::ErrorCode` in the shared model.
pub fn incomplete_reason(status: u32) -> &'static str {
    match status {
        1001 => "Your email address is not verified yet. Check your inbox, or run `si auth resend`.",
        1002 => "Your account is locked.",
        1003 => "Your account is not active yet.",
        _ => "Your account is not ready to sign in yet. Finish setting it up at https://sigit.si/auth.",
    }
}
