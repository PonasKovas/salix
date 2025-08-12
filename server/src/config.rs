use serde::Deserialize;
use std::{net::SocketAddr, path::Path};
use tokio::fs::read_to_string;

#[derive(Deserialize)]
pub struct Config {
	/// Socket to which bind the server
	pub bind_to: SocketAddr,
	/// Postgres database url
	pub database_url: String,
}

pub async fn read_config(path: impl AsRef<Path>) -> anyhow::Result<Config> {
	Ok(toml::from_str(&read_to_string(path).await?)?)
}
