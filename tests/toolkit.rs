use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use unifai_sdk::{
    rig::tool::Tool,
    serde::{Deserialize, Serialize},
    serde_json::{json, Value},
    toolkit::{
        Action, ActionContext, ActionDefinition, ActionParams, ActionResult, ToolkitInfo,
        ToolkitService,
    },
    tools::{CallTool, CallToolArgs, SearchTools, SearchToolsArgs},
};

struct EchoSlam;

#[derive(Serialize, Deserialize)]
#[serde(crate = "serde")]
struct EchoSlamArgs {
    pub content: String,
}

#[derive(Debug, Error)]
#[error("Echo error")]
struct EchoSlamError;

impl Action for EchoSlam {
    const NAME: &'static str = "echo";

    type Error = EchoSlamError;
    type Args = EchoSlamArgs;
    type Output = String;

    async fn definition(&self) -> ActionDefinition {
        ActionDefinition {
            description: "Echo the message".to_string(),
            payload: json!({
                "content": {
                    "type": "string",
                    "description": "The content to echo.",
                    "required": true
                }
            }),
            payment: None,
        }
    }

    async fn call(
        &self,
        ctx: ActionContext,
        params: ActionParams<Self::Args>,
    ) -> Result<ActionResult<Self::Output>, Self::Error> {
        let output = format!(
            "You are agent <${}>, you said \"{}\".",
            ctx.agent_id, params.payload.content
        );

        Ok(ActionResult {
            payload: output,
            payment: None,
        })
    }
}

#[tokio::test]
async fn test_toolkit() {
    tracing_subscriber::fmt().init();

    let unifai_agent_api_key =
        env::var("UNIFAI_AGENT_API_KEY").expect("UNIFAI_AGENT_API_KEY not set");
    let unifai_toolkit_api_key =
        env::var("UNIFAI_TOOLKIT_API_KEY").expect("UNIFAI_TOOLKIT_API_KEY not set");

    let mut service = ToolkitService::new(&unifai_toolkit_api_key);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let unique_toolkit_name = format!("test_echo_{timestamp}");

    service
        .update_info(ToolkitInfo {
            name: unique_toolkit_name.clone(),
            description: "What's in, what's out.".to_string(),
        })
        .await
        .unwrap();

    service.add_action(EchoSlam);

    let _ = service.start().await.unwrap();

    let action_name = {
        let search_tools = SearchTools::new(&unifai_agent_api_key);
        let search_result = search_tools
            .call(SearchToolsArgs {
                query: unique_toolkit_name.clone(),
                limit: None,
            })
            .await
            .unwrap();

        let search_result: Value = serde_json::from_str(&search_result).unwrap();

        search_result
            .as_array()
            .unwrap()
            .iter()
            .find_map(|action| {
                let action_name = action["action"].as_str().unwrap();
                if action_name.contains(&unique_toolkit_name) {
                    Some(action_name.to_string())
                } else {
                    None
                }
            })
            .unwrap()
    };

    let call_tool = CallTool::new(&unifai_agent_api_key);
    let response = call_tool
        .call(CallToolArgs {
            action: action_name,
            payload: json!({
                "content": "How are you".to_string(),
            }),
            payment: None,
        })
        .await
        .unwrap();

    assert!(response.contains("How are you"));
}
