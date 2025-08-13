use std::time::Duration;

use crate::ServerState;
use crate::protocol_util::WebSocketExt;
use anyhow::Result;
use axum::extract::ws::{CloseFrame, Utf8Bytes, WebSocket};
use axum::{
	extract::{Path, State, WebSocketUpgrade, ws::Message},
	http::StatusCode,
	response::IntoResponse,
};
use protocol::s2c;
use protocol::{C2S, ReadMessage};
use thiserror::Error;
use tokio::select;
use tokio::time::{Instant, sleep_until};
use tracing::{error, info};

const PING_INTERVAL: Duration = Duration::from_secs(10);
const PING_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn main_endpoint(
	ws: WebSocketUpgrade,
	Path(version): Path<u32>,
	State(state): State<ServerState>,
) -> impl IntoResponse {
	match version {
		protocol::VERSION => Ok(ws.on_upgrade(move |socket| async move {
			if let Err(e) = handle_socket(state, socket).await {
				error!("{e}");
			}
		})),
		other => Err((
			StatusCode::NOT_IMPLEMENTED,
			format!(
				"Protocol version v{other} not supported. Server running v{}",
				protocol::VERSION
			),
		)),
	}
}

async fn handle_socket(state: ServerState, mut socket: WebSocket) -> Result<(), axum::Error> {
	let mut last_ping_sent = Instant::now();
	let mut last_pong_received = Instant::now();
	let mut waiting_for_pong = false;
	loop {
		select! {
			_ = sleep_until(last_ping_sent + PING_INTERVAL) => {
				last_ping_sent = Instant::now();
				waiting_for_pong = true;
				socket.ping().await?;
			},
			_ = sleep_until(last_pong_received + PING_TIMEOUT), if waiting_for_pong => {
				error!("timed out");
				socket.close().await?;
				break;
			},
			frame = socket.recv() => {
				let frame = match frame {
					Some(x) => x?,
					None => break,
				};

				let bytes = match frame {
					Message::Binary(x) => x,
					Message::Pong(payload) => {
						if !payload.is_empty() {
							error!("invalid pong payload: {payload:?}");
							socket.close().await?;
							break;
						}

						last_pong_received = Instant::now();
						continue;
					}
					Message::Text(text) => {
						error!("unexpected text frame in websocket. dropping connection. {text:?}");
						socket.close().await?;
						break;
					}
					// pings/close frames from client are handled by axum automatically
					_ => continue,
				};

				let parsed = match C2S::read(&bytes) {
					Ok(x) => x,
					Err(_) => {
						error!("invalid packet received");
						socket.send_packet(s2c::Error::InvalidPacket).await?;
						socket.send(Message::Close(None)).await?;
						break;
					}
				};

				if let Err(e) = handle_packet(&state, &mut socket, parsed).await {
					error!("{e}");
					let error_to_send = match e {
						Error::Internal(_) => s2c::Error::Internal,
						Error::Axum(_) => break,
					};

					socket.send_packet(error_to_send).await?;
				}
			},
		}
	}

	Ok(())
}

#[derive(Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Internal(#[from] anyhow::Error),
	#[error(transparent)]
	Axum(#[from] axum::Error),
}

async fn handle_packet(
	state: &ServerState,
	socket: &mut WebSocket,
	packet: C2S,
) -> Result<(), Error> {
	match packet {
		C2S::Authenticate(authenticate) => {
			println!("authenticate {authenticate:?}");
		}
	}

	Ok(())
}
