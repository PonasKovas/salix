use anyhow::{Context, Result};
use std::env;

pub struct TestOptions {
	pub db_name: String,
	pub db_user: Option<String>,
	pub db_password: Option<String>,
	pub db_socket: String,
	pub proxied_db_socket: String,
	pub toxiproxy_control_url: String,
	pub toxiproxy_db_listen_socket: String,
	pub toxiproxy_db_upstream_socket: String,
}

impl TestOptions {
	pub fn get() -> Result<Self> {
		fn var(s: &'static str) -> Result<String> {
			env::var(s).context(s)
		}

		let db_socket = var("TEST_DB_SOCKET")?;
		let proxied_db_socket = var("TEST_PROXIED_DB_SOCKET")?;

		let toxiproxy_db_listen_socket =
			var("TEST_TOXIPROXY_DB_LISTEN_SOCKET").unwrap_or(proxied_db_socket.clone());
		let toxiproxy_db_upstream_socket =
			var("TEST_TOXIPROXY_DB_UPSTREAM_SOCKET").unwrap_or(db_socket.clone());

		Ok(Self {
			db_name: var("TEST_DB_NAME")?,
			db_user: var("TEST_DB_USER").ok(),
			db_password: var("TEST_DB_PASSWORD").ok(),
			db_socket,
			proxied_db_socket,
			toxiproxy_control_url: var("TEST_TOXIPROXY_CONTROL_URL")?,
			toxiproxy_db_listen_socket,
			toxiproxy_db_upstream_socket,
		})
	}
	pub fn db_url(&self, proxied: bool) -> String {
		let mut db_url = "postgres://".to_owned();

		if let Some(user) = &self.db_user {
			db_url.push_str(user);
			if let Some(password) = &self.db_password {
				db_url.push(':');
				db_url.push_str(password);
			}

			db_url.push('@');
		}

		db_url.push_str(&format!(
			"{}/{}",
			if proxied {
				&self.proxied_db_socket
			} else {
				&self.db_socket
			},
			self.db_name
		));

		db_url
	}
}
