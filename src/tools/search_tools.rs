use crate::{constants::DEFAULT_BACKEND_API_ENDPOINT, utils::build_api_client};
use reqwest::Client;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

/// A tool used to search tools on Unifai server.
pub struct SearchTools {
    api_client: Client,
}

impl SearchTools {
    pub fn new(api_key: &str) -> Self {
        let api_client = build_api_client(api_key);
        Self { api_client }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SearchToolsArgs {
    pub query: String,
    pub limit: Option<usize>,
}

impl Tool for SearchTools {
    const NAME: &'static str = "search_services";

    type Error = reqwest::Error;
    type Args = SearchToolsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search for tools. The tools cover a wide range of domains include data source, API, SDK, etc. Try searching whenever you need to use a tool.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                  "query": {
                    "type": "string",
                    "description": "The query to search for tools, you can describe what you want to do or what tools you want to use"
                  },
                  "limit": {
                    "type": "number",
                    "description": "The maximum number of tools to return, must be between 1 and 100, default is 10, recommend at least 10"
                  }
                },
                "required": ["query"],
              }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let endpoint = env::var("UNIFAI_BACKEND_API_ENDPOINT")
            .unwrap_or(DEFAULT_BACKEND_API_ENDPOINT.to_string());
        let url = format!("{endpoint}/actions/search");

        self.api_client
            .get(url)
            .query(&args)
            .send()
            .await?
            .text()
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::{SearchTools, SearchToolsArgs};
    use rig::tool::Tool;
    use serde_json::Value;
    use std::env;

    #[tokio::test]
    async fn test_search_tools_api() {
        let unifai_agent_api_key =
            env::var("UNIFAI_AGENT_API_KEY").expect("UNIFAI_AGENT_API_KEY not set");
        let search_tools = SearchTools::new(&unifai_agent_api_key);

        let response = search_tools
            .call(SearchToolsArgs {
                query: "solana".to_string(),
                limit: Some(10),
            })
            .await
            .unwrap();

        let response: Value = serde_json::from_str(&response).unwrap();

        assert!(response.as_array().unwrap().len() == 10);
    }
}
