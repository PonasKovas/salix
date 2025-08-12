use axum::{
	Error,
	extract::ws::{Message, WebSocket},
};
use protocol::{S2C, WriteMessage};

pub trait WebSocketExt {
	async fn send<T>(&mut self, msg: T) -> Result<(), Error>
	where
		T: Into<S2C>;
	async fn ping(&mut self) -> Result<(), Error>;
}

impl WebSocketExt for WebSocket {
	async fn send<T>(&mut self, msg: T) -> Result<(), Error>
	where
		T: Into<S2C>,
	{
		let msg = msg.into();
		let bytes = msg.write();

		self.send(Message::Binary(bytes.into())).await
	}
	async fn ping(&mut self) -> Result<(), Error> {
		self.send(Message::Ping(Vec::new().into())).await
	}
}
