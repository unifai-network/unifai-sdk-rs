use crate::{constants::DEFAULT_BACKEND_API_ENDPOINT, utils::build_api_client};
use reqwest::Client;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, time::Duration};

/// A tool used to call specific tool on Unifai server.
pub struct CallTool {
    api_client: Client,
}

impl CallTool {
    pub fn new(api_key: &str) -> Self {
        let api_client = build_api_client(api_key);
        Self { api_client }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CallToolArgs {
    pub action: String,
    pub payload: Value,
    pub payment: Option<u64>,
}

impl Tool for CallTool {
    const NAME: &'static str = "invoke_service";

    type Error = reqwest::Error;
    type Args = CallToolArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Call a tool returned by search_services".to_string(),
            parameters: json!({
              "type": "object",
              "properties": {
                "action": {
                  "type": "string",
                  "description": "The exact action you want to call in the search_services result."
                },
                "payload": {
                  "type": "string",
                  "description": "Action payload, based on the payload schema in the search_services result. You can pass either the json object directly or json encoded string of the object.",
                },
                "payment": {
                  "type": "number",
                  "description": "Amount to authorize in USD. Positive number means you will be charged no more than this amount, negative number means you are requesting to get paid for at least this amount. Only include this field if the action you are calling includes payment information.",
                }
              },
              "required": ["action", "payload"],
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let endpoint = env::var("UNIFAI_BACKEND_API_ENDPOINT")
            .unwrap_or(DEFAULT_BACKEND_API_ENDPOINT.to_string());
        let url = format!("{endpoint}/actions/call");

        self.api_client
            .post(url)
            .json(&args)
            .timeout(Duration::from_millis(50_000))
            .send()
            .await?
            .text()
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::{CallTool, CallToolArgs};
    use rig::tool::Tool;
    use serde_json::{json, Value};
    use std::env;

    #[tokio::test]
    async fn test_call_tool_api() {
        let unifai_agent_api_key =
            env::var("UNIFAI_AGENT_API_KEY").expect("UNIFAI_AGENT_API_KEY not set");
        let call_tool = CallTool::new(&unifai_agent_api_key);

        let response = call_tool
            .call(CallToolArgs {
                action: "Solana/7/getBalance".to_string(),
                payload: json!({
                    "walletAddress": "11111111111111111111111111111111"
                }),
                payment: None,
            })
            .await
            .unwrap();

        let response: Value = serde_json::from_str(&response).unwrap();

        assert!(response["payload"]
            .as_str()
            .unwrap()
            .contains("Balance of SOL"));
    }
}
