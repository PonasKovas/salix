use anyhow::Result;
use axum::{
	Error, Router,
	extract::{Path, State, WebSocketUpgrade, ws::WebSocket},
	http::StatusCode,
	response::IntoResponse,
	routing::any,
};
use sqlx::PgPool;
use std::env;

// mod auth;
// mod auth_verify;
// mod version_check;

#[derive(Clone)]
struct ServerState {
	db: PgPool,
}

#[tokio::main]
async fn main() -> Result<()> {
	let db = PgPool::connect(&env::var("DATABASE_URL")?).await?;

	let state = ServerState { db };

	let app = Router::new()
		.route("/v/{version}", any(ws_handler))
		.with_state(state);

	let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
	axum::serve(listener, app).await.unwrap();

	Ok(())
}

async fn ws_handler(
	ws: WebSocketUpgrade,
	Path(version): Path<u32>,
	State(state): State<ServerState>,
) -> impl IntoResponse {
	// might want to support multiple versions later
	match version {
		protocol::VERSION => Ok(ws.on_upgrade(move |socket| async move {
			if let Err(e) = handle_socket(state, socket).await {
				eprintln!("error: {e}");
			}
		})),
		other => Err((
			StatusCode::NOT_FOUND,
			format!(
				"Protocol version v{other} not supported. Server running v{}",
				protocol::VERSION
			),
		)),
	}
}

async fn handle_socket(state: ServerState, mut socket: WebSocket) -> Result<(), Error> {
	while let Some(frame) = socket.recv().await {
		let frame = frame?;
		println!("{frame:?}");
	}

	Ok(())
}
