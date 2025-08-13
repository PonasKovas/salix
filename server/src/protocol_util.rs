use axum::extract::ws::{Message, WebSocket};
use protocol::{C2S, ReadMessage, S2C, WriteMessage};
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
	pub async fn send_packet<T>(&mut self, msg: T) -> Result<(), axum::Error>
	where
		T: Into<S2C>,
	{
		let msg = msg.into();
		let bytes = msg.write();

		self.socket.send(Message::Binary(bytes.into())).await
	}
	pub async fn close(&mut self) -> Result<(), axum::Error> {
		self.socket.send(Message::Close(None)).await
	}
	pub async fn recv(&mut self) -> Result<C2S, RecvError> {
		loop {
			select! {
				_ = self.ping_interval.tick() => {
					self.last_ping_sent = Some(Instant::now());
					self.socket.send(Message::Ping(Vec::new().into())).await?;
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
						Message::Binary(x) => x,
						Message::Pong(_) => {
							self.last_ping_sent = None;
							continue;
						}
						Message::Text(_) => {
							return Err(RecvError::TextFrame);
						}
						// pings/close frames from client are handled by axum automatically
						_ => continue,
					};

					let packet = C2S::read(&packet_bytes).map_err(|_| RecvError::InvalidPacket)?;

					return Ok(packet);
				},
			}
		}
	}
}
