use anyhow::Result;
use server::{cmd_args::Args, config::Config, database::Database, update_listener::UpdateListener};
use std::path::PathBuf;

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

#[tokio::test]
async fn msg_listener_seq_id_race_condition() -> Result<()> {
	let toxiproxy = Toxiproxy::new().await?;

	let db = Database::init(&config(), &args()).await?;
	let listener = UpdateListener::init(&config(), &args(), &db).await?;

	toxiproxy.disrupt().await?;

	toxiproxy.finish().await?;

	Ok(())
}

const TOXIPROXY_BASE_URL: &str = "http://localhost:8474";
const TOXIPROXY_PROXY_NAME: &str = "salix_test_msg_listener_seq_id_race_condition_postgres";
const TOXIPROXY_PROXY_PORT: u16 = 5433;

struct Toxiproxy {
	client: reqwest::Client,
}

impl Toxiproxy {
	async fn new() -> Result<Self> {
		let client = reqwest::Client::new();

		#[derive(serde::Serialize)]
		struct Request<'a> {
			name: &'a str,
			listen: &'a str,
			upstream: &'a str,
		}

		client
			.post(format!("{TOXIPROXY_BASE_URL}/proxies"))
			.body(serde_json::to_string(&Request {
				name: TOXIPROXY_PROXY_NAME,
				listen: &format!("localhost:{TOXIPROXY_PROXY_PORT}"),
				upstream: "localhost:5432",
			})?)
			.send()
			.await?;

		Ok(Self { client })
	}
	async fn finish(self) -> Result<()> {
		self.client
			.delete(format!(
				"{TOXIPROXY_BASE_URL}/proxies/{TOXIPROXY_PROXY_NAME}"
			))
			.send()
			.await?;

		Ok(())
	}
	async fn disrupt(&self) -> Result<()> {
		#[derive(serde::Serialize)]
		struct Request {
			enabled: bool,
		}

		let builder = self.client.post(format!(
			"{TOXIPROXY_BASE_URL}/proxies/{TOXIPROXY_PROXY_NAME}"
		));

		builder
			.try_clone()
			.unwrap()
			.body(serde_json::to_string(&Request { enabled: false })?)
			.send()
			.await?;

		builder
			.try_clone()
			.unwrap()
			.body(serde_json::to_string(&Request { enabled: true })?)
			.send()
			.await?;

		Ok(())
	}
}
