use futures::SinkExt;
use protocol::{C2S, WriteMessage};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::{
	WebSocketStream,
	tungstenite::{Error, Message},
};

pub trait WebSocketExt {
	async fn send_packet<T>(&mut self, msg: T) -> Result<(), Error>
	where
		T: Into<C2S>;
	async fn ping(&mut self) -> Result<(), Error>;
}

impl<S: AsyncRead + AsyncWrite + Unpin> WebSocketExt for WebSocketStream<S> {
	async fn send_packet<T>(&mut self, msg: T) -> Result<(), Error>
	where
		T: Into<C2S>,
	{
		let msg = msg.into();
		let bytes = msg.write();
		SinkExt::send(self, Message::Binary(bytes.into())).await
	}
	async fn ping(&mut self) -> Result<(), Error> {
		SinkExt::send(self, Message::Ping(Vec::new().into())).await
	}
}
