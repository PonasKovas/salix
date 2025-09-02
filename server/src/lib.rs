use anyhow::Result;
use axum::{Router, routing::any};
use clap::Parser;
use database::Database;
use endpoints::{auth::auth_routes, main::main_endpoint};
use logging::init_logging;
use sqlx::PgPool;
use tracing::info;
use update_listener::UpdateListener;

pub mod cmd_args;
pub mod config;
pub mod database;
pub mod endpoints;
pub mod logging;
pub mod populate;
pub mod socket;
pub mod update_listener;

#[derive(Clone)]
pub struct ServerState {
	pub db: Database<PgPool>,
	pub updates: UpdateListener,
}

pub async fn main() -> Result<()> {
	init_logging();

	let args = cmd_args::Args::parse();
	let config = config::read_config(&args.config).await?;

	let db = Database::init(&config, &args).await?;

	if args.populate {
		info!("Populating database.");
		return populate::populate(&db).await;
	}

	let state = ServerState {
		updates: UpdateListener::init(&db).await?,
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
