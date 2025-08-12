use anyhow::Result;
use futures::SinkExt;
use protocol::{C2S, WriteMessage, c2s::Hello};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<()> {
	let (mut ws_stream, _) = connect_async("ws://127.0.0.1:3000/v/1").await?;

	ws_stream
		.send(Message::Binary(
			C2S::from(Hello {
				message: format!("HELLO!!!"),
			})
			.write()
			.into(),
		))
		.await?;

	Ok(())
}
