// Prevent console window in Windows release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result};

mod crate_version;
mod ui;

fn main() -> Result<()> {
	ui::entry_window()?;

	Ok(())
}

// #[tokio::main]
// async fn main() -> Result<()> {
// 	let token: Uuid = std::env::var("AUTH_TOKEN")?.parse()?;

// 	let (mut ws_stream, _) = connect_async("ws://127.0.0.1:3000/v1").await?;

// 	ws_stream
// 		.send_packet(Authenticate {
// 			auth_token: *token.as_bytes(),
// 		})
// 		.await?;

// 	let user_info: UserInfo = ws_stream.recv_packet().await?.context("user info packet")?;
// 	println!("[S] [INFO] my username: {}", user_info.username);

// 	let mut stdin = BufReader::new(stdin());
// 	let mut line_buf = String::new();
// 	loop {
// 		tokio::select! {
// 			packet = ws_stream.recv_packet() => {
// 				let packet = match packet? {
// 					Some(x) => x,
// 					None => break,
// 				};

// 				match packet {
// 					S2C::Error(error) => {
// 						println!("[S] [INFO] SERVER ERROR: {error}");
// 					},
// 					S2C::NewMessage(new_message) => {
// 						println!("[S] {}: {}", new_message.user, new_message.message);
// 					},
// 				}
// 			}
// 			r = stdin.read_line(&mut line_buf) => {
// 				r?;
// 				ws_stream.send_packet(SendMessage{ message: line_buf.trim().to_owned() }).await?;
// 			},
// 		}
// 	}

// 	ws_stream.close(None).await?;

// 	Ok(())
// }
