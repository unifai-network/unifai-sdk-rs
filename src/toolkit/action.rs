use super::{errors::ToolkitError, messages::ActionCallParams};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, future::Future, pin::Pin};

/// A struct used to define an action.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionDefinition {
    pub description: String,
    pub payload: Value,
    pub payment: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct ActionContext<T: for<'a> Deserialize<'a>> {
    pub action: String,
    pub action_id: u64,
    pub agent_id: u64,
    pub payload: T,
    pub payment: Option<u64>,
}

impl From<ActionCallParams> for ActionContext<Value> {
    fn from(value: ActionCallParams) -> Self {
        Self {
            action: value.action,
            action_id: value.action_id,
            agent_id: value.agent_id,
            payload: value.payload,
            payment: value.payment,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionResult<T> {
    #[serde(rename = "payload")]
    pub output: T,
    pub payment: Option<u64>,
}

/// Trait that represents an action of Toolkit
///
/// # Example
/// ```no_run
/// use serde::{Deserialize, Serialize};
/// use serde_json::json;
/// use thiserror::Error;
/// use unifai_sdk::{toolkit::{Action, ActionContext, ActionDefinition, ActionResult}};
///
/// struct EchoSlam;
///
/// #[derive(Serialize, Deserialize)]
/// struct EchoSlamArgs {
///     pub content: String,
/// }
///
/// #[derive(Debug, Error)]
/// #[error("Echo error")]
/// struct EchoSlamError;
///
/// impl Action for EchoSlam {
///     const NAME: &'static str = "echo";
///
///     type Error = EchoSlamError;
///     type Args = EchoSlamArgs;
///     type Output = String;
///
///     async fn definition(&self) -> ActionDefinition {
///         ActionDefinition {
///             description: "Echo the message".to_string(),
///             payload: json!({
///                 "content": {
///                     "type": "string",
///                     "description": "The content to echo.",
///                     "required": true
///                 }
///             }),
///             payment: None,
///         }
///     }
///
///     async fn call(
///         &self,
///         ctx: ActionContext<Self::Args>,
///     ) -> Result<ActionResult<Self::Output>, Self::Error> {
///         let output = format!(
///             "You are agent <${}>, you said \"{}\".",
///             ctx.agent_id, ctx.payload.content
///         );
///
///         Ok(ActionResult {
///             output,
///             payment: None,
///         })
///     }
/// }
///
/// ```
pub trait Action: Sized + Send + Sync {
    /// The name of the action. This name should be unique.
    const NAME: &'static str;

    /// The error type of the action.
    type Error: Error + Send + Sync + 'static;
    /// The arguments type of the action.
    type Args: for<'a> Deserialize<'a> + Send + Sync;
    /// The output type of the action.
    type Output: Serialize;

    /// A method returning the name of the action.
    fn name(&self) -> String {
        Self::NAME.to_string()
    }

    /// A method returning the action definition.
    fn definition(&self) -> impl Future<Output = ActionDefinition> + Send + Sync;

    /// The action execution method.
    fn call(
        &self,
        ctx: ActionContext<Self::Args>,
    ) -> impl Future<Output = Result<ActionResult<Self::Output>, Self::Error>> + Send + Sync;
}

pub(crate) trait ActionDyn: Send + Sync {
    fn name(&self) -> String;

    fn definition(&self) -> Pin<Box<dyn Future<Output = ActionDefinition> + Send + Sync + '_>>;

    fn call(
        &self,
        ctx: ActionContext<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<ActionResult<Value>, ToolkitError>> + Send + Sync + '_>>;
}

impl<T: Action> ActionDyn for T {
    fn name(&self) -> String {
        self.name()
    }

    fn definition(&self) -> Pin<Box<dyn Future<Output = ActionDefinition> + Send + Sync + '_>> {
        Box::pin(<Self as Action>::definition(self))
    }

    fn call(
        &self,
        ctx: ActionContext<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<ActionResult<Value>, ToolkitError>> + Send + Sync + '_>>
    {
        Box::pin(async move {
            let payload = serde_json::from_value(ctx.payload)?;
            let ctx = ActionContext {
                action: ctx.action,
                action_id: ctx.action_id,
                agent_id: ctx.agent_id,
                payload,
                payment: ctx.payment,
            };

            <Self as Action>::call(self, ctx)
                .await
                .map_err(|e| ToolkitError::ActionCallError(Box::new(e)))
                .and_then(|result| {
                    Ok(ActionResult {
                        output: serde_json::to_value(result.output)?,
                        payment: result.payment,
                    })
                })
        })
    }
}
