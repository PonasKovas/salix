use axum::extract::ws::{Message as WSMessage, WebSocket};
use protocol::{C2S, IntoMessage, Message, S2C};
use std::{future::pending, time::Duration};
use thiserror::Error;
use tokio::{
	select,
	time::{Instant, Interval, MissedTickBehavior, interval, sleep_until},
};

const PING_INTERVAL: Duration = Duration::from_secs(10);
const PING_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Socket<'a> {
	socket: &'a mut WebSocket,
	ping_interval: Interval,
	last_ping_sent: Option<Instant>,
}

#[derive(Error, Debug)]
pub enum RecvError {
	#[error(transparent)]
	Axum(#[from] axum::Error),
	#[error("timed out")]
	TimedOut,
	#[error("websocket closed")]
	Closed,
	#[error("unexpected text frame")]
	TextFrame,
	#[error("invalid packet")]
	InvalidPacket,
}

impl<'a> Socket<'a> {
	pub fn new(socket: &'a mut WebSocket) -> Self {
		let mut ping_interval = interval(PING_INTERVAL);
		ping_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
		Self {
			socket,
			ping_interval,
			last_ping_sent: None,
		}
	}
	pub async fn send_packet(&mut self, msg: impl IntoMessage<S2C>) -> Result<(), axum::Error> {
		let bytes = msg.into_message().write();

		self.socket.send(WSMessage::Binary(bytes.into())).await
	}
	pub async fn close(&mut self) -> Result<(), axum::Error> {
		self.socket.send(WSMessage::Close(None)).await
	}
	pub async fn recv<P: Message<C2S>>(&mut self) -> Result<P, RecvError> {
		loop {
			select! {
				_ = self.ping_interval.tick() => {
					self.last_ping_sent = Some(Instant::now());
					self.socket.send(WSMessage::Ping(Vec::new().into())).await?;
				},
				_ = async {
					match self.last_ping_sent {
						Some(last_ping) => sleep_until(last_ping + PING_TIMEOUT).await,
						None => pending().await,
					}
				} => {
					return Err(RecvError::TimedOut);
				},
				frame = self.socket.recv() => {
					let frame = match frame {
						Some(x) => x?,
						None => return Err(RecvError::Closed),
					};

					let packet_bytes = match frame {
						WSMessage::Binary(x) => x,
						WSMessage::Pong(_) => {
							self.last_ping_sent = None;
							continue;
						}
						WSMessage::Text(_) => {
							return Err(RecvError::TextFrame);
						}
						// pings/close frames from client are handled by axum automatically
						_ => continue,
					};

					let packet = P::read(&packet_bytes).map_err(|_| RecvError::InvalidPacket)?;

					return Ok(packet);
				},
			}
		}
	}
}
