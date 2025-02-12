use super::{
    action::{ActionDyn, ActionResult},
    errors::Result,
    messages::{ActionCallParams, ActionCallResult, ActionsRegisterParams, ToolkitMessage},
    Action,
};
use crate::{
    constants::{BACKEND_WS_ENDPOINT, FRONTEND_API_ENDPOINT},
    utils::build_api_client,
};
use futures_util::{future::join_all, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{net::TcpStream, spawn, sync::mpsc::unbounded_channel, task::JoinHandle, time::sleep};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Bytes, Message},
    MaybeTlsStream, WebSocketStream,
};

const PING_INTERVAL: Duration = Duration::from_millis(30_000);

#[derive(Serialize, Deserialize)]
pub struct ToolkitInfo {
    pub name: String,
    pub description: String,
}

/// A service that manages and runs a Toolkit.
///
/// # Example
/// ```ignore
/// let unifai_toolkit_api_key = "UNIFAI_TOOLKIT_API_KEY";
///
/// let mut service = ToolkitService::new(unifai_toolkit_api_key.to_string());
///
/// let info = ToolkitInfo {
///     name: "Echo Slam".to_string(),
///     description: "What's in, what's out.".to_string(),
/// };
///
/// service.update_info(info).await.unwrap();
///
/// service.add_action(EchoSlam);
///
/// let runner = service.start().await.unwrap();
/// let _ = runner.await.unwrap();
/// ```
pub struct ToolkitService {
    api_key: String,
    actions: HashMap<String, Box<dyn ActionDyn>>,
}

impl ToolkitService {
    /// Create a Toolkit service with Unifai API Key.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            actions: HashMap::new(),
        }
    }

    /// Update Toolkit's name and description.
    pub async fn update_info(&self, info: ToolkitInfo) -> Result<()> {
        let client = build_api_client(self.api_key.clone());
        let url = format!("{FRONTEND_API_ENDPOINT}/toolkits/fields/");

        client.post(url).json(&info).send().await?;

        Ok(())
    }

    /// Add an action that implements the [Action] trait to be registered when starting.
    pub fn add_action(&mut self, action: impl Action + 'static) {
        self.actions.insert(action.name(), Box::new(action));
    }

    /// Start the Toolkit service asynchronously.
    ///
    /// Once the service is ready, it returns a [JoinHandle] that keeps the service alive.
    pub async fn start(self) -> Result<JoinHandle<Result<()>>> {
        let url = format!(
            "{BACKEND_WS_ENDPOINT}?type=toolkit&api-key={}",
            self.api_key
        );

        let (mut ws_stream, _) = connect_async(url).await?;

        // Register actions
        {
            let actions = HashMap::from_iter(
                join_all(
                    self.actions
                        .values()
                        .map(|action| async { (action.name(), action.definition().await) }),
                )
                .await,
            );
            let message = ToolkitMessage::RegisterActions {
                data: ActionsRegisterParams { actions },
            };

            ws_stream
                .send(Message::text(serde_json::to_string(&message)?))
                .await?;
        }

        let runner = spawn(self.run_continuously(ws_stream));

        Ok(runner)
    }

    async fn run_continuously(
        self,
        mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<()> {
        let (response_sender, mut response_receiver) = unbounded_channel();

        let actions_arc = Arc::new(self.actions);

        loop {
            tokio::select! {
                _ = sleep(PING_INTERVAL) => {
                    ws_stream.send(Message::Ping(Bytes::new())).await.unwrap_or_else(|e| {
                        tracing::error!("Failed to send pong: {:?}", e);
                    });
                }

                Some(msg) = response_receiver.recv() => {
                    ws_stream.send(Message::text(serde_json::to_string(&msg)?)).await.unwrap_or_else(|e| {
                        tracing::error!("Failed to send response: {:?}", e);
                    });
                }

                Some(msg) = ws_stream.next() => {
                    match msg {
                        Ok(Message::Text(text)) => match serde_json::from_str::<ToolkitMessage>(&text) {
                            Ok(ToolkitMessage::Action { data }) => {
                                let actions_arc = actions_arc.clone();
                                let response_sender = response_sender.clone();
                                spawn(async move {
                                    let action_name = data.action.clone();
                                    tracing::info!("Action call: {:?}", data);
                                    if let Some(result) = handle_action_call(actions_arc, data).await {
                                        tracing::info!("Action call result: {:?}", result);
                                        let result_msg = ToolkitMessage::ActionResult { data: result };
                                        response_sender.send(result_msg).unwrap();
                                    } else {
                                        tracing::warn!("Action not found: {}", action_name);
                                    }
                                });
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!("Received unknown message: {:?}", e);
                            }
                        },
                        Ok(Message::Ping(data)) => {
                            ws_stream.send(Message::Pong(data)).await?;
                        }
                        Ok(Message::Close(_)) => break,
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("Failed to parse message: {:?}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

async fn handle_action_call(
    actions: Arc<HashMap<String, Box<dyn ActionDyn>>>,
    params: ActionCallParams,
) -> Option<ActionCallResult> {
    if let Some(action) = actions.get(&params.action) {
        let output = action.call(params.clone().into()).await.unwrap_or_else(|e| {
            tracing::debug!("Error occured during action call: {:?}", e);
            ActionResult {
            output: json!({ "error": "An unexpected error occurred, please report to the toolkit developer" }),
            payment: None,
        }});

        Some(ActionCallResult {
            action: params.action,
            action_id: params.action_id,
            agent_id: params.agent_id,
            payload: output.output,
            payment: output.payment,
        })
    } else {
        None
    }
}
