#[derive(Debug, thiserror::Error)]
pub enum ToolkitError {
    #[error("ActionCallError: {0}")]
    ActionCallError(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("JsonError: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("ApiError: {0}")]
    ApiError(#[from] reqwest::Error),

    #[error("WebSocketError: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
}

pub type Result<T> = std::result::Result<T, ToolkitError>;
