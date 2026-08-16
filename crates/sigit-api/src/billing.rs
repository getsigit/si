//! Plan, cloud entitlement, and the hosted checkout / portal links.

use crate::{
    client::Client,
    error::Result,
    models::{Billing, BillingUrl},
};

impl Client {
    /// `GET /api/v1/billing`
    pub async fn billing(&self) -> Result<Billing> {
        self.get("billing").await
    }

    /// `POST /api/v1/billing/checkout` — a Stripe Checkout URL to open in a
    /// browser. Purchases deliberately happen on the web, never in the client.
    pub async fn billing_checkout(&self, plan: &str) -> Result<String> {
        let body = serde_json::json!({ "plan": plan });
        let response: BillingUrl = self.post("billing/checkout", &body).await?;
        Ok(response.url)
    }

    /// `POST /api/v1/billing/portal` — the Stripe customer portal URL.
    pub async fn billing_portal(&self) -> Result<String> {
        let response: BillingUrl = self.post("billing/portal", &serde_json::json!({})).await?;
        Ok(response.url)
    }
}
