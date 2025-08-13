use crate::ServerState;
use crate::protocol_util::{RecvError, Socket};
use anyhow::{Context, Result};
use axum::extract::ws::WebSocket;
use axum::{
	extract::{Path, State, WebSocketUpgrade, ws::Message},
	http::StatusCode,
	response::IntoResponse,
};
use protocol::s2c::{self, UserInfo};
use protocol::{C2S, ReadMessage};
use sqlx::postgres::PgListener;
use std::future::pending;
use std::time::Duration;
use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc::{Receiver, channel};
use tokio::time::{Instant, Interval, MissedTickBehavior, interval, sleep_until};
use tracing::error;
use uuid::Uuid;

pub async fn main_endpoint(
	ws: WebSocketUpgrade,
	Path(version): Path<u32>,
	State(server): State<ServerState>,
) -> impl IntoResponse {
	match version {
		protocol::VERSION => Ok(ws.on_upgrade(move |mut socket| async move {
			let mut socket = Socket::new(&mut socket);

			if let Err(e) = authenticate(&server, &mut socket).await {
				error!("{e}");

				'send_err_to_client: {
					let error_to_send_client: s2c::Error = match e {
						Error::Axum(_) => break 'send_err_to_client,
						other => other.into(),
					};

					let _ = socket.send_packet(error_to_send_client).await;
					let _ = socket.close().await;
				}
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
	user_id: Uuid,
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
	#[error("unexpected packet")]
	UnexpectedPacket,
	#[error("while receiving: {0}")]
	Recv(#[from] RecvError),
}
impl From<sqlx::Error> for Error {
	fn from(value: sqlx::Error) -> Self {
		Self::Internal(anyhow::Error::new(value))
	}
}
impl From<Error> for s2c::Error {
	fn from(value: Error) -> Self {
		match value {
			Error::Axum(_) => Self::Internal,
			Error::Internal(_) => Self::Internal,
			Error::Unauthenticated => Self::Unauthenticated,
			Error::Unauthorized => Self::Unauthorized,
			Error::UnexpectedPacket => Self::UnexpectedPacket,
			Error::Recv(recv_error) => match recv_error {
				RecvError::Axum(_) => Self::Internal,
				RecvError::TimedOut => Self::TimedOut,
				RecvError::Closed => Self::Internal,
				RecvError::TextFrame => Self::TextFrame,
				RecvError::InvalidPacket => Self::InvalidPacket,
			},
		}
	}
}

async fn authenticate(server: &ServerState, socket: &mut Socket<'_>) -> Result<(), Error> {
	// first and foremost we are waiting for the Authenticate packet
	// this is a loop so we can skip pings (why would the client send a ping before an actual packet tho?)
	let first_packet = socket.recv().await?;

	let user_id: Uuid;
	match first_packet {
		C2S::Authenticate(authenticate) => {
			let token = Uuid::from_bytes(authenticate.auth_token);

			let row = sqlx::query!(
				r#"SELECT users.id, users.username FROM active_sessions JOIN users ON active_sessions.user_id = users.id WHERE active_sessions.token = $1"#,
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

			user_id = row.id;
			socket
				.send_packet(UserInfo {
					username: row.username,
				})
				.await?;
		}
		_ => return Err(Error::Unauthenticated),
	}

	// great, now can start normal handling

	let state = ConnectionState { user_id };

	let new_msgs = new_msg_listener(&server).await?;

	handle_socket(server, state, socket, new_msgs).await
}

async fn handle_socket(
	server: &ServerState,
	mut state: ConnectionState,
	socket: &mut Socket<'_>,
	mut new_msgs: Receiver<NewMsgListenerUpdate>,
) -> Result<(), Error> {
	loop {
		select! {
			packet = socket.recv() => {
				let packet = packet?;

				if let Err(e) = handle_packet(server, &mut state, socket, packet).await {
					error!("{e}");
					let error_to_send_client: s2c::Error = match e {
						Error::Axum(_) => break, // axum errors are not recoverable
						other => other.into(),
					};

					socket.send_packet(error_to_send_client).await?;
				}
			}
			msg = new_msgs.recv() => {
				match msg.context("new msg listener task dropped?")? {
					NewMsgListenerUpdate::Disconnect => {
						// fetch all recent messages manually
						// TODO
					},
					NewMsgListenerUpdate::NewMsg(uuid) => {
						let row = sqlx::query!(
							r#"SELECT u.id AS user_id, u.username, m.message FROM messages AS m JOIN users AS u ON m.user_id = u.id WHERE m.id = $1"#,
							uuid,
						)
						.fetch_one(&server.db)
						.await?;

						socket.send_packet(s2c::NewMessage{
							user: row.username,
							message: row.message,
						}).await?;
					},
				}
			},
		}
	}

	Ok(())
}

async fn handle_packet(
	server: &ServerState,
	state: &mut ConnectionState,
	socket: &mut Socket<'_>,
	packet: C2S,
) -> Result<(), Error> {
	match packet {
		C2S::Authenticate(_) => return Err(Error::UnexpectedPacket),
		C2S::SendMessage(send_message) => {
			let msg_id = Uuid::now_v7();
			sqlx::query!(
				r#"INSERT INTO messages (id, user_id, message, sent_at) VALUES ($1, $2, $3, NOW())"#,
				msg_id,
				state.user_id,
				send_message.message
			)
			.execute(&server.db)
			.await?;
		}
	}

	Ok(())
}

enum NewMsgListenerUpdate {
	NewMsg(Uuid),
	// sent when the connection to the db was interrupted and there may be missed messages
	Disconnect,
}

async fn new_msg_listener(
	server: &ServerState,
) -> Result<Receiver<NewMsgListenerUpdate>, sqlx::Error> {
	let mut new_msg_listener = PgListener::connect_with(&server.db).await?;
	new_msg_listener.listen("chat").await?;

	let (sender, receiver) = channel(5);

	tokio::spawn(async move {
		loop {
			match new_msg_listener.try_recv().await {
				Ok(Some(notification)) => {
					let uuid = notification.payload().parse().unwrap();
					if sender
						.send(NewMsgListenerUpdate::NewMsg(uuid))
						.await
						.is_err()
					{
						break;
					}
				}
				// None is returned when the connection is interrupted
				Ok(None) => {
					if sender.send(NewMsgListenerUpdate::Disconnect).await.is_err() {
						break;
					}
				}
				Err(e) => {
					error!("listening to notify updates: {e}");
					break;
				}
			}
		}
	});

	Ok(receiver)
}
