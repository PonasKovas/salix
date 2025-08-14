use crate::protocol_util::{RecvError, Socket};
use crate::{ServerState, db};
use anyhow::{Context, Result};
use axum::{
	extract::{Path, State, WebSocketUpgrade},
	http::StatusCode,
	response::IntoResponse,
};
use protocol::C2S;
use protocol::s2c::{self, UserInfo};
use sqlx::postgres::PgListener;
use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc::{Receiver, channel};
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

			if let Err(e) = handle_socket(&server, &mut socket).await {
				error!("{e}");

				let error_to_send_client: s2c::Error = match e {
					Error::Axum(_) => return, // axum error very bad, dont even try sending
					other => other.into(),
				};

				let _ = socket.send_packet(error_to_send_client).await;
				let _ = socket.close().await;
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
	last_msg_seq_id: Option<i64>,
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
	#[error("timed out")]
	TimedOut,
	#[error("websocket closed")]
	Closed,
	#[error("unexpected text frame")]
	TextFrame,
	#[error("invalid packet")]
	InvalidPacket,
}
impl From<sqlx::Error> for Error {
	fn from(value: sqlx::Error) -> Self {
		Self::Internal(anyhow::Error::new(value))
	}
}
impl From<RecvError> for Error {
	fn from(value: RecvError) -> Self {
		match value {
			RecvError::Axum(e) => Self::Axum(e),
			RecvError::TimedOut => Self::TimedOut,
			RecvError::Closed => Self::Closed,
			RecvError::TextFrame => Self::TextFrame,
			RecvError::InvalidPacket => Self::InvalidPacket,
		}
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
			Error::TimedOut => Self::TimedOut,
			Error::Closed => Self::Internal,
			Error::TextFrame => Self::TextFrame,
			Error::InvalidPacket => Self::InvalidPacket,
		}
	}
}

async fn handle_socket(server: &ServerState, socket: &mut Socket<'_>) -> Result<(), Error> {
	// first and foremost we are waiting for the Authenticate packet
	let first_packet = socket.recv().await?;

	let user_id: Uuid = match first_packet {
		C2S::Authenticate(authenticate) => {
			let token = Uuid::from_bytes(authenticate.auth_token);

			let user = db::User::by_auth_token(token)
				.fetch_optional(&server.db)
				.await?
				.ok_or(Error::Unauthorized)?;

			socket
				.send_packet(UserInfo {
					username: user.username,
				})
				.await?;
			user.id
		}
		_ => return Err(Error::Unauthenticated),
	};

	// great, now can start normal stuff

	let mut state = ConnectionState {
		user_id,
		last_msg_seq_id: None,
	};
	let mut new_msgs = new_msg_listener(&server).await?;
	loop {
		next_event(server, &mut state, socket, &mut new_msgs).await?;
	}
}

async fn next_event(
	server: &ServerState,
	state: &mut ConnectionState,
	socket: &mut Socket<'_>,
	new_msgs: &mut Receiver<NewMsgListenerUpdate>,
) -> Result<(), Error> {
	select! {
		packet = socket.recv() => {
			handle_packet(server, state, socket, packet?).await?;
		}
		msg = new_msgs.recv() => {
			match msg.context("new msg listener task dropped?")? {
				NewMsgListenerUpdate::Disconnect => {
					// fetch all recent messages manually
					if let Some(last_msg_id) = state.last_msg_seq_id {
						for msg in fetch_messages_since(server, last_msg_id).await? {
							socket.send_packet(s2c::NewMessage{
								user: msg.username,
								message: msg.message,
							}).await?;
						}
					}
				},
				NewMsgListenerUpdate::NewMsg(uuid) => {
					let message = fetch_message(server, uuid).await?;

					state.last_msg_seq_id = Some(message.sequence_id);

					socket.send_packet(s2c::NewMessage{
						user: message.username,
						message: message.message,
					}).await?;
				},
			}
		},
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
			insert_message(server, state.user_id, &send_message.message).await?;
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

// sql helpers

struct Message {
	user_id: Uuid,
	username: String,
	sequence_id: i64,
	message: String,
}

async fn fetch_message(server: &ServerState, message_id: Uuid) -> Result<Message, Error> {
	let message = sqlx::query_as!(
		Message,
		r#"SELECT u.id AS user_id, u.username, m.message, m.sequence_id FROM messages AS m JOIN users AS u ON m.user_id = u.id WHERE m.id = $1"#,
		message_id,
	)
	.fetch_one(&server.db)
	.await?;

	Ok(message)
}

async fn insert_message(server: &ServerState, user_id: Uuid, message: &str) -> Result<Uuid, Error> {
	let msg_id = Uuid::now_v7();
	sqlx::query!(
		r#"INSERT INTO messages (id, user_id, message, sent_at) VALUES ($1, $2, $3, NOW())"#,
		msg_id,
		user_id,
		message
	)
	.execute(&server.db)
	.await?;

	Ok(msg_id)
}

async fn fetch_messages_since(server: &ServerState, seq_id: i64) -> Result<Vec<Message>, Error> {
	let messages = sqlx::query_as!(
		Message,
		r#"SELECT u.id AS user_id, u.username, m.message, m.sequence_id FROM messages AS m JOIN users AS u ON m.user_id = u.id WHERE m.sequence_id > $1"#,
		seq_id,
	)
	.fetch_all(&server.db)
	.await?;

	Ok(messages)
}
