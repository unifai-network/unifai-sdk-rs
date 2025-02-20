use super::Result;
use crate::constants::DEFAULT_TRANSACTION_API_ENDPOINT;
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::env;

#[derive(Clone, Debug)]
pub struct ActionContext {
    pub(crate) api_client: Client,
    pub action: String,
    pub action_id: u64,
    pub agent_id: u64,
}

impl ActionContext {
    pub async fn create_transaction(
        &self,
        tx_type: &str,
        payload: impl Serialize,
    ) -> Result<Value> {
        let endpoint = env::var("UNIFAI_TRANSACTION_API_ENDPOINT")
            .unwrap_or(DEFAULT_TRANSACTION_API_ENDPOINT.to_string());
        let url = format!("{endpoint}/tx/create");

        let args = json!({
            "agentId": self.agent_id,
            "actionId": self.action_id,
            "actionName": self.action,
            "type": tx_type,
            "payload": payload,
        });

        let result = self
            .api_client
            .post(url)
            .json(&args)
            .send()
            .await?
            .json()
            .await?;

        Ok(result)
    }
}
