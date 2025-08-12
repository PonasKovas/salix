use anyhow::Result;
use auth::auth_routes;
use axum::{
	Router,
	extract::{
		Path, State, WebSocketUpgrade,
		ws::{Message, WebSocket},
	},
	http::StatusCode,
	response::IntoResponse,
	routing::any,
};
use clap::Parser;
use logging::init_logging;
use protocol::{C2S, ReadMessage};
use sqlx::PgPool;
use tracing::{error, info};

mod auth;
mod cmd_args;
mod config;
mod logging;
mod protocol_util;

#[derive(Clone)]
struct ServerState {
	db: PgPool,
}

#[tokio::main]
async fn main() -> Result<()> {
	init_logging();

	let args = cmd_args::Args::parse();
	let config = config::read_config(&args.config).await?;

	let db = PgPool::connect(&config.database_url).await?;

	if !args.no_migrate {
		sqlx::migrate!().run(&db).await?;
	}

	let state = ServerState { db };

	let app = Router::new()
		.nest("/auth", auth_routes())
		.route("/v{version}", any(ws_handler))
		.with_state(state);

	let listener = tokio::net::TcpListener::bind(config.bind_to).await?;
	info!("TCP listener bound on {}", listener.local_addr()?);
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
				error!("{e}");
			}
		})),
		other => Err((
			StatusCode::NOT_IMPLEMENTED,
			format!(
				"Protocol version v{other} not supported. Server running v{}",
				protocol::VERSION
			),
		)),
	}
}

async fn handle_socket(state: ServerState, mut socket: WebSocket) -> Result<()> {
	while let Some(frame) = socket.recv().await {
		let frame = frame?;
		if let Message::Binary(bytes) = &frame {
			let parsed = C2S::read(bytes)?;
			info!("{:?}", parsed);
		}
		info!("{frame:?}");
	}

	Ok(())
}
