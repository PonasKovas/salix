use crate::ServerState;
use crate::socket::{RecvError, Socket};
use crate::update_listener::UpdateSubscriber;
use anyhow::{Context, Result};
use axum::{
	extract::{Path, State, WebSocketUpgrade},
	http::StatusCode,
	response::IntoResponse,
};
use protocol::C2S;
use protocol::c2s::Authenticate;
use protocol::s2c::{self, UserInfo};
use sqlx::postgres::PgListener;
use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc::{Receiver, channel};
use tokio_pubsub::PubSubMessage;
use tracing::error;
use uuid::Uuid;

pub async fn main_endpoint(
	ws: WebSocketUpgrade,
	Path(version): Path<u32>,
	State(mut server): State<ServerState>,
) -> impl IntoResponse {
	match version {
		protocol::VERSION => Ok(ws.on_upgrade(move |mut socket| async move {
			let mut socket = Socket::new(&mut socket);

			if let Err(e) = handle_socket(&mut server, &mut socket).await {
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
	update_subscriber: UpdateSubscriber,
}

#[derive(Error, Debug)]
enum Error {
	#[error(transparent)]
	Internal(#[from] anyhow::Error),
	#[error(transparent)]
	Axum(#[from] axum::Error),
	#[error("invalid auth token")]
	Unauthorized,
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
			Error::Unauthorized => Self::Unauthorized,
			Error::TimedOut => Self::TimedOut,
			Error::Closed => Self::Internal,
			Error::TextFrame => Self::TextFrame,
			Error::InvalidPacket => Self::InvalidPacket,
		}
	}
}

async fn handle_socket(server: &mut ServerState, socket: &mut Socket<'_>) -> Result<(), Error> {
	// first and foremost we are waiting for the Authenticate packet
	let auth: Authenticate = socket.recv().await?;
	let token = Uuid::from_bytes(auth.auth_token);

	let user = server
		.db
		.user_by_auth_token(token)
		.await?
		.ok_or(Error::Unauthorized)?;

	socket
		.send_packet(UserInfo {
			username: user.username,
		})
		.await?;

	// great, now can start normal stuff

	let mut state = ConnectionState {
		user_id: user.id,
		last_msg_seq_id: None,
		update_subscriber: server.updates.subscribe().await,
	};

	state
		.update_subscriber
		.subscribe_chat(Uuid::from_u128(5))
		.await
		.unwrap();
	loop {
		next_event(server, &mut state, socket).await?;
	}
}

async fn next_event(
	server: &mut ServerState,
	state: &mut ConnectionState,
	socket: &mut Socket<'_>,
) -> Result<(), Error> {
	select! {
		packet = socket.recv() => {
			handle_packet(server, state, socket, packet?).await?;
		}
		msg = state.update_subscriber.recv_chat_messages() => {
			let msg = msg?;

			state.last_msg_seq_id = Some(msg.sequence_id);

			socket.send_packet(s2c::NewMessage{
				user: msg.user_id.to_string(),
				message: msg.message.clone(),
			}).await?;
		},
	}

	Ok(())
}

async fn handle_packet(
	server: &mut ServerState,
	state: &mut ConnectionState,
	socket: &mut Socket<'_>,
	packet: C2S,
) -> Result<(), Error> {
	match packet {
		C2S::SendMessage(send_message) => {
			server
				.db
				.insert_message(Uuid::from_u128(5), state.user_id, &send_message.message)
				.await?;
		}
	}

	Ok(())
}
