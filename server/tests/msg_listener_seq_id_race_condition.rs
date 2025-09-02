use anyhow::{Result, bail, ensure};
use common::TestOptions;
use server::{
	cmd_args::Args, config::Config, database::Database, populate, update_listener::UpdateListener,
};
use sqlx::{PgPool, postgres::PgListener};
use std::{path::PathBuf, time::Duration};
use tokio::time::timeout;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod common;

fn config(options: &TestOptions) -> Config {
	Config {
		bind_to: "0.0.0.0:0".parse().unwrap(),
		database_url: options.db_url(false),
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
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::builder()
				.with_default_directive(LevelFilter::ERROR.into())
				.from_env()
				.expect("invalid logging directives"),
		)
		.pretty()
		.init();

	let options = TestOptions::get()?;

	let args = args();
	let config = config(&options);

	let toxiproxy = Toxiproxy::new(&options).await?;

	// Database::init also migrates automatically
	let db = Database::init(&config, &args).await?;
	eprintln!("direct db connected");
	let proxied_db = Database::new(PgPool::connect(&options.db_url(true)).await?);
	eprintln!("proxied db connected");

	populate::populate(&db).await?;

	assert_disruption(&toxiproxy, &proxied_db).await?;

	let listener = UpdateListener::init(&proxied_db).await?;
	let mut subscriber = listener.subscribe().await;
	subscriber.messages.add_topic(populate::CHAT_ID).await?;

	let mut transaction_a = Database::new(db.begin().await?);
	let mut transaction_b = Database::new(db.begin().await?);

	// A starts adding a new message first
	transaction_a
		.insert_message(populate::CHAT_ID, populate::USER_A_ID, "message A")
		.await?;

	// B adds a message later
	transaction_b
		.insert_message(populate::CHAT_ID, populate::USER_B_ID, "message B")
		.await?;

	// B commits first
	transaction_b.inner.commit().await?;

	// then listener connection is disrupted
	toxiproxy.disable().await?;

	// then A commits
	transaction_a.inner.commit().await?;

	// after A is commited, listener connection is restored
	toxiproxy.enable().await?;

	// try to receive the 2 messages with our subscriber
	for expected_msg in ["message B", "message A"] {
		match timeout(Duration::from_millis(100), subscriber.messages.recv()).await {
			Ok(x) => {
				let (channel, message) = x?;

				ensure!(
					channel == populate::CHAT_ID,
					"received message notification from wrong channel? what"
				);

				let msg = match message {
					tokio_pubsub::PubSubMessage::Ok(x) => x,
					tokio_pubsub::PubSubMessage::Lagged(_) => bail!("receiver lagged "),
				};

				ensure!(msg.message == expected_msg, "wrong order?");
			}
			Err(_) => bail!("subscriber recv timed out. A message got permanently lost probably"),
		}
	}

	subscriber.destroy().await;

	toxiproxy.finish().await?;
	Ok(())
}

// assert that disrupting the toxiproxy proxy affects the DB connection
async fn assert_disruption(toxiproxy: &Toxiproxy, db: &Database<PgPool>) -> Result<()> {
	let mut listener = PgListener::connect_with(db).await?;

	toxiproxy.disrupt().await?;

	match timeout(Duration::from_millis(100), listener.try_recv()).await {
		Ok(x) => match x? {
			Some(_) => bail!("what?? not even listening to any channel"),
			None => {} // expected - connection disrupted
		},
		Err(_) => bail!("recv timed out. Is the DB connection through toxiproxy?"),
	}

	Ok(())
}

// TOXIPROXY stuff
////////////////

const TOXIPROXY_PROXY_NAME: &str = "salix_test_msg_listener_seq_id_race_condition_postgres";

struct Toxiproxy {
	client: reqwest::Client,
	base_url: String,
}

#[derive(serde::Serialize, Default)]
struct ToxiproxyProxyFields<'a> {
	#[serde(skip_serializing_if = "Option::is_none")]
	name: Option<&'a str>,
	#[serde(skip_serializing_if = "Option::is_none")]
	listen: Option<&'a str>,
	#[serde(skip_serializing_if = "Option::is_none")]
	upstream: Option<&'a str>,
	#[serde(skip_serializing_if = "Option::is_none")]
	enabled: Option<bool>,
}

impl Toxiproxy {
	async fn new(options: &TestOptions) -> Result<Self> {
		let client = reqwest::Client::new();

		let base_url = options.toxiproxy_control_url.clone();

		// first cleanup if there was already a proxy with that name
		client
			.delete(format!("{base_url}/proxies/{TOXIPROXY_PROXY_NAME}"))
			.send()
			.await?;

		client
			.post(format!("{base_url}/proxies"))
			.body(serde_json::to_string(&ToxiproxyProxyFields {
				name: Some(TOXIPROXY_PROXY_NAME),
				listen: Some(&options.toxiproxy_db_listen_socket),
				upstream: Some(&options.toxiproxy_db_upstream_socket),
				..Default::default()
			})?)
			.send()
			.await?
			.error_for_status()?;

		Ok(Self { client, base_url })
	}
	async fn finish(self) -> Result<()> {
		self.client
			.delete(format!("{}/proxies/{TOXIPROXY_PROXY_NAME}", self.base_url))
			.send()
			.await?
			.error_for_status()?;

		Ok(())
	}
	async fn disable(&self) -> Result<()> {
		self.client
			.post(format!("{}/proxies/{TOXIPROXY_PROXY_NAME}", self.base_url))
			.body(serde_json::to_string(&ToxiproxyProxyFields {
				enabled: Some(false),
				..Default::default()
			})?)
			.send()
			.await?
			.error_for_status()?;

		Ok(())
	}
	async fn enable(&self) -> Result<()> {
		self.client
			.post(format!("{}/proxies/{TOXIPROXY_PROXY_NAME}", self.base_url))
			.body(serde_json::to_string(&ToxiproxyProxyFields {
				enabled: Some(true),
				..Default::default()
			})?)
			.send()
			.await?
			.error_for_status()?;

		Ok(())
	}
	async fn disrupt(&self) -> Result<()> {
		self.disable().await?;
		self.enable().await?;

		Ok(())
	}
}
