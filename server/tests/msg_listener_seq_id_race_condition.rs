use anyhow::{Result, anyhow};
use server::{cmd_args::Args, config::Config, database::Database, update_listener::UpdateListener};
use std::path::PathBuf;

#[tokio::test]
async fn msg_listener_seq_id_race_condition() -> Result<()> {
	init_toxiproxy().await?;

	let db = Database::init(&config(), &args()).await?;
	let listener = UpdateListener::init(&config(), &args(), &db).await?;

	Ok(())
}

async fn init_toxiproxy() -> Result<()> {
	let client = reqwest::Client::new();

	#[derive(serde::Serialize)]
	struct Request<'a> {
		name: &'a str,
		listen: &'a str,
		upstream: &'a str,
	}

	// client
	// 	.delete("http://localhost:8474/proxies/postgres")
	// 	.send()
	// 	.await?;

	client
		.post("http://localhost:8474/proxies")
		.body(serde_json::to_string(&Request {
			name: "postgres",
			listen: "localhost:5433",
			upstream: "localhost:5432",
		})?)
		.send()
		.await?;

	Ok(())
}

fn config() -> Config {
	Config {
		bind_to: "0.0.0.0:0".parse().unwrap(),
		database_url: "postgres://localhost:5433/salix".to_owned(),
	}
}

fn args() -> Args {
	Args {
		config: PathBuf::new(),
		no_migrate: false,
		populate: false,
	}
}
