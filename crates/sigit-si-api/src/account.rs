//! Sign in, sign out, and the current-user profile.

use crate::{
    client::Client,
    error::Result,
    models::{AccountStatus, User},
};
use reqwest::Method;

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

    /// `POST /api/v1/auth/sign_up` — creates an account and sends the
    /// confirmation email. The answer is the sign-up result as the server sent
    /// it (`code`, `message`, `data`), so it is returned untyped.
    pub async fn sign_up(&self, email: &str, password: &str) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "email": email.trim().to_ascii_lowercase(),
            "password": password,
        });
        self.post("auth/sign_up", &body).await
    }

    /// `POST /api/v1/auth/confirmation/resend` — sends the confirmation email
    /// again. The server answers the same way whether or not the address has an
    /// account.
    pub async fn resend_confirmation(&self, email: &str) -> Result<()> {
        let body = serde_json::json!({ "email": email.trim().to_ascii_lowercase() });
        self.send_empty(
            self.request(Method::POST, "auth/confirmation/resend")
                .json(&body),
        )
        .await
    }

    /// `POST /api/v1/auth/password/reset` — emails reset instructions. The
    /// server answers the same way whether or not the address has an account.
    pub async fn reset_password(&self, email: &str) -> Result<()> {
        self.send_empty(self.reset_password_request(email)).await
    }

    fn reset_password_request(&self, email: &str) -> reqwest::RequestBuilder {
        let body = serde_json::json!({ "email": email.trim().to_ascii_lowercase() });
        self.request(Method::POST, "auth/password/reset")
            .json(&body)
    }

    /// `DELETE /api/v1/me` — permanently deletes the signed-in account.
    pub async fn delete_account(&self) -> Result<()> {
        self.delete("me").await
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

#[cfg(test)]
mod tests {
    use {super::*, crate::Environment};

    #[test]
    fn a_reset_posts_the_normalized_email() {
        let client = Client::builder(Environment::Production)
            .base_url("https://sigit.si")
            .build()
            .unwrap();
        let request = client
            .reset_password_request("  Seto@Example.com ")
            .build()
            .unwrap();

        assert_eq!(request.method(), Method::POST);
        assert_eq!(
            request.url().as_str(),
            "https://sigit.si/api/v1/auth/password/reset"
        );
        let body = request.body().and_then(|b| b.as_bytes()).unwrap();
        let body: serde_json::Value = serde_json::from_slice(body).unwrap();
        assert_eq!(body["email"], "seto@example.com");
    }
}
