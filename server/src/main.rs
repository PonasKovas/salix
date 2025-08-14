use anyhow::Result;
use axum::{Router, routing::any};
use clap::Parser;
use endpoints::{auth::auth_routes, main::main_endpoint};
use logging::init_logging;
use sqlx::PgPool;
use tracing::info;

mod cmd_args;
mod config;
mod db;
mod endpoints;
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
		.route("/v{version}", any(main_endpoint))
		.with_state(state);

	let listener = tokio::net::TcpListener::bind(config.bind_to).await?;

	info!("TCP listener bound on {}", listener.local_addr()?);
	axum::serve(listener, app).await.unwrap();

	Ok(())
}
