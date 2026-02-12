//! WebSocket upgrade handler.

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::Response;
use filehub_realtime::connection::authenticator::WsAuthenticator;
use filehub_realtime::message::OutboundMessage;
use futures::{SinkExt, StreamExt};
use tracing::{info, warn};

use filehub_core::error::AppError;

use crate::state::AppState;

/// Query parameter for WebSocket authentication.
#[derive(Debug, serde::Deserialize)]
pub struct WsQuery {
    /// JWT access token.
    pub token: String,
}

/// GET /ws?token={jwt} — WebSocket upgrade
pub async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
) -> Result<Response, AppError> {
    // Authenticate before upgrade
    let authenticator = WsAuthenticator::new(state.jwt_decoder.clone());
    let auth_info = authenticator.authenticate(&query.token).await?;

    Ok(ws.on_upgrade(move |socket| handle_ws_connection(state, auth_info, socket)))
}

/// Handles an established WebSocket connection.
async fn handle_ws_connection(
    state: AppState,
    auth: filehub_realtime::connection::authenticator::WsAuthUser,
    socket: WebSocket,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<OutboundMessage>(100);

    // Register connection
    let handle = match state.realtime.connections.register(
        auth.user_id,
        auth.session_id,
        auth.role.clone(),
        auth.username.clone(),
        tx,
    ) {
        Some(h) => h,
        None => {
            warn!(user_id = %auth.user_id, "Connection limit reached, rejecting");
            return;
        }
    };

    let conn_id = handle.id;

    info!(
        conn_id = %conn_id,
        user_id = %auth.user_id,
        "WebSocket connection established"
    );

    // Spawn outbound message forwarder
    let outbound_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(text) => {
                    if ws_tx.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to serialize outbound message");
                }
            }
        }
    });

    // Process inbound messages
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(Message::Text(text)) => {
                state
                    .realtime
                    .connections
                    .handle_inbound(&conn_id, &text)
                    .await;
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Ok(Message::Ping(_)) => {
                // Ping is handled by axum automatically
            }
            Ok(_) => {}
            Err(e) => {
                warn!(conn_id = %conn_id, error = %e, "WebSocket error");
                break;
            }
        }
    }

    // Cleanup
    outbound_task.abort();
    state.realtime.connections.unregister(conn_id);

    info!(
        conn_id = %conn_id,
        user_id = %auth.user_id,
        "WebSocket connection closed"
    );
}
