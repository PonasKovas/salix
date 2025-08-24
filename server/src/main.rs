use anyhow::Result;
use axum::{Router, routing::any};
use clap::Parser;
use database::Database;
use endpoints::{auth::auth_routes, main::main_endpoint};
use logging::init_logging;
use sqlx::PgPool;
use tracing::info;
use update_listener::UpdateListener;

mod cmd_args;
mod config;
mod database;
mod endpoints;
mod logging;
mod socket;
mod update_listener;

#[derive(Clone)]
struct ServerState {
	db: Database<PgPool>,
	updates: UpdateListener,
}

#[tokio::main]
async fn main() -> Result<()> {
	init_logging();

	let args = cmd_args::Args::parse();
	let config = config::read_config(&args.config).await?;

	let db = Database::init(&config, &args).await?;
	let state = ServerState {
		updates: UpdateListener::init(&config, &args, &db).await?,
		db,
	};

	let app = Router::new()
		.nest("/auth", auth_routes())
		.route("/v{version}", any(main_endpoint))
		.with_state(state);

	let listener = tokio::net::TcpListener::bind(config.bind_to).await?;

	info!("TCP listener bound on {}", listener.local_addr()?);
	axum::serve(listener, app).await?;

	Ok(())
}
