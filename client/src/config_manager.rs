use crate::{
	InnerClient,
	config::{Config, ConfigError, save_config},
};
use std::sync::Arc;
use url::Url;

#[derive(Debug, Clone)]
pub struct ConfigManager {
	pub(super) inner: Arc<InnerClient>,
}

impl ConfigManager {
	pub fn with<F, R>(&self, reader: F) -> R
	where
		F: FnOnce(&Config) -> R,
	{
		let config_guard = self.inner.config.lock().unwrap();
		reader(&config_guard)
	}
	pub async fn update<F>(&self, updater: F) -> Result<(), ConfigError>
	where
		F: FnOnce(&mut Config),
	{
		let config_clone: Config;

		{
			let mut config_guard = self.inner.config.lock().unwrap();

			updater(&mut config_guard);

			config_clone = config_guard.clone();
		}

		save_config(&config_clone).await?;

		Ok(())
	}
	pub fn get_server_url(&self) -> Url {
		self.with(|config| config.server_url.clone())
	}
	pub async fn set_server_url(&self, url: Url) -> Result<(), ConfigError> {
		self.update(move |config| config.server_url = url).await
	}
}
