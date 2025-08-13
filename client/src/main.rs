use anyhow::Result;
use futures::{SinkExt, StreamExt};
use protocol::{
	ReadMessage, S2C,
	c2s::{Authenticate, SendMessage},
};
use protocol_util::WebSocketExt;
use tokio::io::{AsyncBufReadExt, BufReader, stdin};
use tokio_tungstenite::{connect_async, tungstenite::Message};
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

	let mut stdin = BufReader::new(stdin());
	let mut line_buf = String::new();
	loop {
		tokio::select! {
			frame = ws_stream.next() => {
				let frame = match frame {
					Some(x) => x?,
					None => break,
				};

				match frame {
					Message::Binary(packet) => {
						let packet = S2C::read(&packet)?;

						match packet {
							S2C::Error(error) => {
								println!("[S] [INFO] SERVER ERROR: {error}");
							},
							S2C::UserInfo(user_info) => {
								println!("[S] [INFO] my username: {}", user_info.username);
							},
							S2C::NewMessage(new_message) => {
								println!("[S] {}: {}", new_message.user, new_message.message);
							},
						}
					}
					Message::Ping(payload) => {
						ws_stream.send(Message::Pong(payload)).await?;
					}
					Message::Close(_) => {
						break;
					}
					_ => {}
				}
			},
			r = stdin.read_line(&mut line_buf) => {
				r?;
				ws_stream.send_packet(SendMessage{ message: line_buf.trim().to_owned() }).await?;
			},
		}
	}

	ws_stream.send(Message::Close(None)).await?;

	Ok(())
}
