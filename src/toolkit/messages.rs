use super::ActionDefinition;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ToolkitMessage {
    Action { data: ActionCallParams },
    ActionResult { data: ActionCallResult },
    RegisterActions { data: ActionsRegisterParams },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionCallParams {
    pub action: String,
    #[serde(rename = "actionID")]
    pub action_id: u64,
    #[serde(rename = "agentID")]
    pub agent_id: u64,
    pub payload: Value,
    pub payment: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionCallResult {
    pub action: String,
    #[serde(rename = "actionID")]
    pub action_id: u64,
    #[serde(rename = "agentID")]
    pub agent_id: u64,
    pub payload: Value,
    pub payment: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionsRegisterParams {
    pub actions: HashMap<String, ActionDefinition>,
}
