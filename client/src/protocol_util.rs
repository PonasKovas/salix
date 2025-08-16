use futures::{SinkExt, StreamExt};
use protocol::{C2S, IntoMessage, Message, S2C};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::{
	WebSocketStream,
	tungstenite::{self, Message as TungsteniteMessage},
};

#[derive(Error, Debug)]
pub enum Error {
	#[error("unexpected ws text frame {0:?}")]
	UnexpectedTextFrame(tungstenite::Utf8Bytes),
	#[error("websocket: {0}")]
	Transport(#[from] tungstenite::Error),
	#[error("invalid packet, {0}")]
	Protocol(#[from] protocol::Error),
}

pub trait WebSocketExt {
	async fn send_packet(&mut self, msg: impl IntoMessage<C2S>) -> Result<(), tungstenite::Error>;
	async fn recv_packet<P>(&mut self) -> Result<Option<P>, Error>
	where
		P: Message<S2C>;
	async fn ping(&mut self) -> Result<(), tungstenite::Error>;
}

impl<S: AsyncRead + AsyncWrite + Unpin> WebSocketExt for WebSocketStream<S> {
	async fn send_packet(&mut self, msg: impl IntoMessage<C2S>) -> Result<(), tungstenite::Error> {
		let bytes = msg.into_message().write();
		SinkExt::send(self, TungsteniteMessage::Binary(bytes.into())).await
	}
	async fn recv_packet<P>(&mut self) -> Result<Option<P>, Error>
	where
		P: Message<S2C>,
	{
		loop {
			let frame = match self.next().await {
				Some(f) => f?,
				None => return Ok(None),
			};

			match frame {
				TungsteniteMessage::Binary(packet) => {
					let packet = P::read(&packet)?;

					return Ok(Some(packet));
				}
				TungsteniteMessage::Ping(payload) => {
					self.send(TungsteniteMessage::Pong(payload)).await?;
					continue;
				}
				TungsteniteMessage::Text(text) => {
					return Err(Error::UnexpectedTextFrame(text));
				}
				TungsteniteMessage::Close(_) => return Ok(None),
				_ => {}
			}
		}
	}
	async fn ping(&mut self) -> Result<(), tungstenite::Error> {
		SinkExt::send(self, TungsteniteMessage::Ping(Vec::new().into())).await
	}
}
