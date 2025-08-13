use crate::ServerState;
use crate::protocol_util::WebSocketExt;
use anyhow::Result;
use axum::extract::ws::WebSocket;
use axum::{
	extract::{Path, State, WebSocketUpgrade, ws::Message},
	http::StatusCode,
	response::IntoResponse,
};
use protocol::c2s::Authenticate;
use protocol::s2c::{self, UserInfo};
use protocol::{C2S, ReadMessage};
use std::time::Duration;
use thiserror::Error;
use tokio::select;
use tokio::time::{Instant, sleep_until};
use tracing::error;
use uuid::Uuid;

const PING_INTERVAL: Duration = Duration::from_secs(10);
const PING_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn main_endpoint(
	ws: WebSocketUpgrade,
	Path(version): Path<u32>,
	State(server): State<ServerState>,
) -> impl IntoResponse {
	match version {
		protocol::VERSION => Ok(ws.on_upgrade(move |socket| async move {
			if let Err(e) = handle_socket(server, socket).await {
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

struct ConnectionState {
	user_id: Option<Uuid>,
}

async fn handle_socket(server: ServerState, mut socket: WebSocket) -> Result<(), axum::Error> {
	let mut state = ConnectionState { user_id: None };

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

				if let Err(e) = handle_packet(&server, &mut state,  &mut socket, parsed).await {
					error!("{e}");
					let error_to_send = match e {
						Error::Internal(_) => s2c::Error::Internal,
						Error::Axum(_) => break,
						Error::Unauthenticated => s2c::Error::Unauthenticated,
						Error::UnexpectedPacket => s2c::Error::UnexpectedPacket,
						Error::Unauthorized => s2c::Error::Unauthorized,
					};

					socket.send_packet(error_to_send).await?;
				}
			},
		}
	}

	Ok(())
}

#[derive(Error, Debug)]
enum Error {
	#[error(transparent)]
	Internal(#[from] anyhow::Error),
	#[error(transparent)]
	Axum(#[from] axum::Error),
	#[error("client not authenticated")]
	Unauthenticated,
	#[error("invalid auth token")]
	Unauthorized,
	#[error("unexpected packet received")]
	UnexpectedPacket,
}

impl From<sqlx::Error> for Error {
	fn from(value: sqlx::Error) -> Self {
		Self::Internal(anyhow::Error::new(value))
	}
}

async fn handle_packet(
	server: &ServerState,
	state: &mut ConnectionState,
	socket: &mut WebSocket,
	packet: C2S,
) -> Result<(), Error> {
	if state.user_id.is_none() {
		if let C2S::Authenticate(authenticate) = packet {
			let user_info = try_authenticate(server, authenticate).await?;
			socket.send_packet(user_info).await?;
		} else {
			return Err(Error::Unauthenticated);
		}
		return Ok(());
	}

	match packet {
		C2S::Authenticate(_) => return Err(Error::UnexpectedPacket),
		C2S::SendMessage(send_message) => {
			//
		}
	}

	Ok(())
}

async fn try_authenticate(
	server: &ServerState,
	authenticate: Authenticate,
) -> Result<UserInfo, Error> {
	let token = Uuid::from_bytes(authenticate.auth_token);

	let row = sqlx::query!(
		r#"SELECT users.username FROM active_sessions JOIN users ON active_sessions.user_id = users.id WHERE active_sessions.token = $1"#,
		token,
	)
	.fetch_one(&server.db)
	.await
	.map_err(|e| {
		if let sqlx::Error::RowNotFound = e {
			Error::Unauthorized
		} else {
			e.into()
		}
	})?;

	Ok(UserInfo {
		username: row.username,
	})
}
