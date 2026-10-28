//! siGit Code Cloud Agent runs.

use {
    crate::{client::Client, error::Result, models::AgentRun},
    reqwest::{Method, RequestBuilder},
    serde::Deserialize,
};

#[derive(Deserialize)]
struct AgentRuns {
    agent_runs: Vec<AgentRun>,
}

impl Client {
    /// `GET /api/v1/agent/runs` — your runs, newest first.
    pub async fn agent_runs(&self) -> Result<Vec<AgentRun>> {
        let runs: AgentRuns = self.get("agent/runs").await?;
        Ok(runs.agent_runs)
    }

    /// `GET /api/v1/agent/runs/:id` — one run, with the tail of its
    /// transcript in `transcript_tail`.
    pub async fn agent_run(&self, id: i64) -> Result<AgentRun> {
        self.get(&format!("agent/runs/{id}")).await
    }

    /// `POST /api/v1/agent/runs` — start a run on a GitHub repository that has
    /// the siGit Code GitHub App installed and that your connected GitHub
    /// account can write to.
    pub async fn create_agent_run(&self, repo_full_name: &str, prompt: &str) -> Result<AgentRun> {
        self.send(self.create_agent_run_request(repo_full_name, prompt))
            .await
    }

    /// `POST /api/v1/agent/runs/:id/cancel`
    pub async fn cancel_agent_run(&self, id: i64) -> Result<AgentRun> {
        self.send(self.request(Method::POST, &format!("agent/runs/{id}/cancel")))
            .await
    }

    /// `GET /api/v1/agent/runs/:id/handoff` — the branch, prompt and command to
    /// carry on with a run on your own machine. Returned as the server sent it.
    pub async fn agent_run_handoff(&self, id: i64) -> Result<serde_json::Value> {
        self.get(&format!("agent/runs/{id}/handoff")).await
    }

    fn create_agent_run_request(&self, repo_full_name: &str, prompt: &str) -> RequestBuilder {
        self.request(Method::POST, "agent/runs")
            .json(&serde_json::json!({
                "repo_full_name": repo_full_name,
                "prompt": prompt,
            }))
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::Environment};

    fn client() -> Client {
        Client::builder(Environment::Production)
            .base_url("https://sigit.si")
            .token("tok")
            .build()
            .unwrap()
    }

    #[test]
    fn a_run_is_started_with_the_repo_and_prompt() {
        let request = client()
            .create_agent_run_request("seto/demo", "Fix the build")
            .build()
            .unwrap();

        assert_eq!(request.method(), Method::POST);
        assert_eq!(request.url().as_str(), "https://sigit.si/api/v1/agent/runs");
        let body = request.body().and_then(|b| b.as_bytes()).unwrap();
        let body: serde_json::Value = serde_json::from_slice(body).unwrap();
        assert_eq!(body["repo_full_name"], "seto/demo");
        assert_eq!(body["prompt"], "Fix the build");
    }

    #[test]
    fn the_list_is_unwrapped() {
        let body = r#"{"agent_runs":[{"id":7,"title":null,"repo_full_name":"seto/demo",
            "trigger_kind":"api","status":"running","cancel_requested":false,
            "exit_reason":null,"base_branch":"main","head_branch":"sigit/x",
            "pull_request_url":null,"issue_number":null,"pr_number":null,
            "inference_requests_used":3,"created_at":"2026-10-01T10:00:00Z",
            "started_at":null,"finished_at":null}]}"#;
        let runs: AgentRuns = serde_json::from_str(body).unwrap();
        assert_eq!(runs.agent_runs[0].id, 7);
        assert!(runs.agent_runs[0].transcript_tail.is_none());
    }

    #[test]
    fn a_single_run_carries_its_transcript_tail() {
        let body = r#"{"id":7,"title":"t","repo_full_name":"seto/demo","trigger_kind":"api",
            "status":"succeeded","cancel_requested":false,"exit_reason":null,
            "base_branch":null,"head_branch":null,"pull_request_url":null,
            "issue_number":null,"pr_number":null,"inference_requests_used":0,
            "created_at":"2026-10-01T10:00:00Z","started_at":null,"finished_at":null,
            "transcript_tail":"line 1\nline 2\n"}"#;
        let run: AgentRun = serde_json::from_str(body).unwrap();
        assert_eq!(run.transcript_tail.as_deref(), Some("line 1\nline 2\n"));
    }
}
