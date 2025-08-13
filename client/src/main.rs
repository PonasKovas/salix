use anyhow::Result;
use futures::StreamExt;
use protocol::c2s::Authenticate;
use protocol_util::WebSocketExt;
use tokio_tungstenite::connect_async;
use uuid::Uuid;

mod protocol_util;

#[tokio::main]
async fn main() -> Result<()> {
	let token: Uuid = std::env::var("AUTH_TOKEN")?.parse()?;

	let (mut ws_stream, _) = connect_async("ws://127.0.0.1:3000/v1").await?;

	ws_stream
		.send_packet(Authenticate {
			auth_token: *token.as_bytes(),
		})
		.await?;

	while let Some(frame) = ws_stream.next().await {
		let frame = frame?;

		println!("{frame:?}");
	}

	// ws_stream.close(None).await?;

	Ok(())
}
