use super::{
    action::{ActionDyn, ActionResult},
    errors::Result,
    messages::{ActionCallParams, ActionCallResult, ActionsRegisterParams, ToolkitMessage},
    Action, ActionContext, ActionParams,
};
use crate::{
    constants::{DEFAULT_BACKEND_WS_ENDPOINT, DEFAULT_FRONTEND_API_ENDPOINT},
    utils::build_api_client,
};
use futures_util::{future::join_all, SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, env, sync::Arc, time::Duration};
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
/// let mut service = ToolkitService::new("UNIFAI_TOOLKIT_API_KEY");
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
    api_client: Client,
    actions: HashMap<String, Box<dyn ActionDyn>>,
}

impl ToolkitService {
    /// Create a Toolkit service with Unifai API Key.
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            api_client: build_api_client(api_key),
            actions: HashMap::new(),
        }
    }

    /// Update Toolkit's name and description.
    pub async fn update_info(&self, info: ToolkitInfo) -> Result<()> {
        let client = build_api_client(&self.api_key);
        let endpoint = env::var("UNIFAI_FRONTEND_API_ENDPOINT")
            .unwrap_or(DEFAULT_FRONTEND_API_ENDPOINT.to_string());
        let url = format!("{endpoint}/toolkits/fields/");

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
        let endpoint = env::var("UNIFAI_BACKEND_WS_ENDPOINT")
            .unwrap_or(DEFAULT_BACKEND_WS_ENDPOINT.to_string());
        let url = format!("{endpoint}?type=toolkit&api-key={}", self.api_key);

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

        tracing::info!("Toolkit service is running");

        let runner = spawn(self.run_continuously(ws_stream));

        Ok(runner)
    }

    async fn run_continuously(
        self,
        mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<()> {
        let (response_sender, mut response_receiver) = unbounded_channel();

        let self_arc = Arc::new(self);

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
                                let self_arc = self_arc.clone();
                                let response_sender = response_sender.clone();

                                spawn(async move {
                                    let action_name = data.action.clone();
                                    tracing::info!("Action call: {:?}", data);

                                    if let Some(result) = handle_action_call(self_arc, data).await {
                                        tracing::info!("Action result: {:?}", result);

                                        response_sender
                                            .send(ToolkitMessage::ActionResult { data: result })
                                            .unwrap();
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
    toolkit: Arc<ToolkitService>,
    params: ActionCallParams,
) -> Option<ActionCallResult> {
    if let Some(action) = toolkit.actions.get(&params.action) {
        let result = action
            .call(
                ActionContext {
                    api_client: toolkit.api_client.clone(),
                    action: params.action.clone(),
                    action_id: params.action_id.clone(),
                    agent_id: params.agent_id.clone(),
                },
                ActionParams {
                    payload: params.payload,
                    payment: params.payment,
                },
            )
            .await
            .unwrap_or_else(|e| {
                tracing::debug!("Error occured during action call: {:?}", e);

                ActionResult {
                    payload: json!({
                        "error": e.to_string()
                    }),
                    payment: None,
                }
            });

        Some(ActionCallResult {
            action: params.action,
            action_id: params.action_id,
            agent_id: params.agent_id,
            payload: result.payload,
            payment: result.payment,
        })
    } else {
        None
    }
}
