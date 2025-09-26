use std::sync::Arc;

use anyhow::Result;
use axum::{Router, routing::any};
use clap::Parser;
use config::Config;
use database::Database;
use email::Email;
use endpoints::{auth::auth_routes, main::main_endpoint};
use logging::init_logging;
use sqlx::PgPool;
use tracing::info;
use update_listener::UpdateListener;

pub mod cmd_args;
pub mod config;
pub mod database;
pub mod email;
pub mod endpoints;
pub mod logging;
pub mod pages;
pub mod populate;
pub mod socket;
pub mod update_listener;

#[derive(Clone)]
pub struct ServerState {
	pub db: Database<PgPool>,
	pub updates: UpdateListener,
	pub email: Email,
	pub config: Arc<Config>,
}

pub async fn main() -> Result<()> {
	init_logging();

	let args = cmd_args::Args::parse();
	let config = Arc::new(config::read_config(&args.config).await?);

	let db = Database::init(&config, &args).await?;

	if args.populate {
		info!("Populating database.");
		return populate::populate(&db).await;
	}

	let listener = tokio::net::TcpListener::bind(config.bind_to).await?;

	let state = ServerState {
		updates: UpdateListener::init(&db).await?,
		db,
		email: Email::init(&config)?,
		config,
	};

	let app = Router::new()
		.nest("/auth", auth_routes())
		.route("/v{version}", any(main_endpoint))
		.with_state(state);

	info!("TCP listener bound on {}", listener.local_addr()?);
	axum::serve(listener, app).await?;

	Ok(())
}
