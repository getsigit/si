//! siGit Code Cloud Sessions — named, resumable conversations.

use crate::{
    client::Client,
    error::Result,
    models::{AppendedMessage, CloudSession},
};

impl Client {
    /// `GET /api/v1/sessions` — most recently updated first.
    pub async fn sessions(&self) -> Result<Vec<CloudSession>> {
        self.get("sessions").await
    }

    /// `GET /api/v1/sessions/:id` — the session plus its full transcript.
    pub async fn session(&self, id: i64) -> Result<CloudSession> {
        self.get(&format!("sessions/{id}")).await
    }

    /// `POST /api/v1/sessions`
    pub async fn create_session(
        &self,
        title: Option<&str>,
        model: Option<&str>,
    ) -> Result<CloudSession> {
        self.post("sessions", &session_body(title, model)).await
    }

    /// `PATCH /api/v1/sessions/:id`
    pub async fn update_session(
        &self,
        id: i64,
        title: Option<&str>,
        model: Option<&str>,
    ) -> Result<CloudSession> {
        self.patch(&format!("sessions/{id}"), &session_body(title, model))
            .await
    }

    /// `DELETE /api/v1/sessions/:id`
    pub async fn delete_session(&self, id: i64) -> Result<()> {
        self.delete(&format!("sessions/{id}")).await
    }

    /// `POST /api/v1/sessions/:id/messages` — append one message.
    pub async fn append_message(
        &self,
        id: i64,
        role: &str,
        content: &str,
    ) -> Result<AppendedMessage> {
        let body = serde_json::json!({ "role": role, "content": content });
        self.post(&format!("sessions/{id}/messages"), &body).await
    }
}

/// The server treats absent keys as "leave unchanged", so only send what the
/// caller actually supplied.
fn session_body(title: Option<&str>, model: Option<&str>) -> serde_json::Value {
    let mut body = serde_json::Map::new();
    if let Some(title) = title {
        body.insert("title".into(), title.into());
    }
    if let Some(model) = model {
        body.insert("model".into(), model.into());
    }
    serde_json::Value::Object(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_fields_stay_out_of_the_update_body() {
        let body = session_body(Some("Refactor auth"), None);
        assert_eq!(body["title"], "Refactor auth");
        assert!(body.get("model").is_none());
    }
}
