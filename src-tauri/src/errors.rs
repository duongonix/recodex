use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl IpcError {
    pub fn new(code: &str, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            recoverable,
        }
    }

    pub fn capture_unavailable() -> Self {
        Self::new(
            "capture_unavailable",
            "Native screen capture is not implemented yet.",
            true,
        )
    }

    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::new("invalid_state", message, true)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal_error", message, true)
    }
}
