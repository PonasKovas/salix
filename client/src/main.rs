use anyhow::Result;
use protocol::c2s::Hello;
use protocol_util::WebSocketExt;
use tokio_tungstenite::connect_async;

mod protocol_util;

#[tokio::main]
async fn main() -> Result<()> {
	let (mut ws_stream, _) = connect_async("ws://127.0.0.1:3000/v1").await?;

	ws_stream
		.send(Hello {
			message: format!("HELLO!!!"),
		})
		.await?;

	ws_stream.close(None).await?;

	Ok(())
}
