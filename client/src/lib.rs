pub use auth::Auth;
use config::Config;
pub use config_manager::ConfigManager;
use std::sync::{Arc, Mutex};

pub mod auth;
pub mod auth_token_store;
pub mod config;
mod config_manager;
mod protocol_util; // todo wtf is this? DELET

#[derive(Debug, Clone)]
pub struct Client {
	inner: Arc<InnerClient>,
	pub config: ConfigManager,
	pub auth: Auth,
}

#[derive(Debug)]
struct InnerClient {
	config: Mutex<Config>,
}

impl Client {
	pub fn with_config(config: Config) -> Self {
		let inner = Arc::new(InnerClient {
			config: Mutex::new(config),
		});

		Self {
			config: ConfigManager {
				inner: Arc::clone(&inner),
			},
			auth: Auth {
				client: Arc::clone(&inner),
			},
			inner,
		}
	}
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
