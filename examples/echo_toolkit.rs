use std::env;
use unifai_sdk::{
    serde::{Deserialize, Serialize},
    serde_json::json,
    thiserror::Error,
    toolkit::{
        Action, ActionContext, ActionDefinition, ActionParams, ActionResult, ToolkitInfo,
        ToolkitService,
    },
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let unifai_toolkit_api_key =
        env::var("UNIFAI_TOOLKIT_API_KEY").expect("UNIFAI_TOOLKIT_API_KEY not set");

    let mut service = ToolkitService::new(&unifai_toolkit_api_key);

    let info = ToolkitInfo {
        name: "Echo Slam".to_string(),
        description: "What's in, what's out.".to_string(),
    };

    service.update_info(info).await.unwrap();

    service.add_action(EchoSlam);

    let runner = service.start().await.unwrap();
    let _ = runner.await.unwrap();
}
