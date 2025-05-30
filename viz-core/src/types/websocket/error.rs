use crate::{Error, IntoResponse, Response, StatusCode, ThisError};

/// Rejects with an error when [`WebSocket`][super::WebSocket] extraction fails.
#[derive(Debug, ThisError)]
pub enum WebSocketError {
    /// Missing `Connection` upgrade header.
    #[error("missing `Connection` upgrade")]
    MissingConnectUpgrade,

    /// Invalid `Connection` upgrade header.
    #[error("invalid `Connection` upgrade")]
    InvalidConnectUpgrade,

    /// Missing `Upgrade` header.
    #[error("missing `Upgrade`")]
    MissingUpgrade,

    /// Invalid `Upgrade` header.
    #[error("invalid `Upgrade`")]
    InvalidUpgrade,

    /// Missing `Sec-WebSocket-Version` header.
    #[error("missing `Sec-WebSocket-Version`")]
    MissingWebSocketVersion,

    /// Invalid `Sec-WebSocket-Version` header.
    #[error("invalid `Sec-WebSocket-Version`")]
    InvalidWebSocketVersion,

    /// Missing `Sec-WebSocket-Key` header.
    #[error("missing `Sec-WebSocket-Key`")]
    MissingWebSocketKey,

    /// Request upgrade required.
    #[error("request upgrade required")]
    ConnectionNotUpgradable,

    /// Transparents [`tokio_tungstenite::tungstenite::Error`].
    #[error(transparent)]
    TungsteniteError(#[from] tokio_tungstenite::tungstenite::Error),
}

impl IntoResponse for WebSocketError {
    fn into_response(self) -> Response {
        (
            match self {
                Self::MissingConnectUpgrade
                | Self::InvalidConnectUpgrade
                | Self::MissingUpgrade
                | Self::InvalidUpgrade
                | Self::MissingWebSocketVersion
                | Self::InvalidWebSocketVersion
                | Self::MissingWebSocketKey => StatusCode::BAD_REQUEST,
                Self::ConnectionNotUpgradable => StatusCode::UPGRADE_REQUIRED,
                Self::TungsteniteError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            },
            self.to_string(),
        )
            .into_response()
    }
}

impl From<WebSocketError> for Error {
    fn from(e: WebSocketError) -> Self {
        e.into_error()
    }
}
