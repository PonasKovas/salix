use serde::{Deserialize, Serialize};
use std::{io::ErrorKind, path::PathBuf};
use thiserror::Error;
use tokio::fs;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	pub server_url: Url,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			server_url: "http://localhost:3000/".parse().unwrap(),
		}
	}
}

#[derive(Debug, Error)]
pub enum ConfigError {
	#[error(
		"Could not determine a valid configuration directory for the application. This may happen in unusual environments where a home directory is not set."
	)]
	NoProjectDirs,

	#[error("Failed to create the configuration directory at {path:?}")]
	CreateConfigDir {
		path: PathBuf,
		#[source]
		source: std::io::Error,
	},

	#[error("Failed to read the configuration file from {path:?}")]
	ReadError {
		path: PathBuf,
		#[source]
		source: std::io::Error,
	},

	#[error("Failed to write the configuration file to {path:?}")]
	WriteError {
		path: PathBuf,
		#[source]
		source: std::io::Error,
	},

	#[error("Failed to parse the configuration file")]
	ParseToml(#[from] toml::de::Error),

	#[error("Failed to serialize the configuration to TOML format")]
	SerializeToml(#[from] toml::ser::Error),
}

fn get_config_path() -> Result<PathBuf, ConfigError> {
	let dirs = directories::ProjectDirs::from("chat.salix", "Salix", "Salix")
		.ok_or(ConfigError::NoProjectDirs)?;

	Ok(dirs.config_dir().join("client.toml"))
}

pub async fn load_config() -> Result<Config, ConfigError> {
	let path = get_config_path()?;

	let raw = fs::read_to_string(&path)
		.await
		.map_err(|e| ConfigError::ReadError { path, source: e })?;

	let parsed = toml::from_str(&raw)?;

	Ok(parsed)
}

pub async fn save_config(config: &Config) -> Result<(), ConfigError> {
	let toml = toml::to_string_pretty(config)?;

	let path = get_config_path()?;

	if let Some(parent_dir) = path.parent() {
		fs::create_dir_all(&parent_dir)
			.await
			.map_err(|e| ConfigError::CreateConfigDir {
				path: parent_dir.to_path_buf(),
				source: e,
			})?;
	}

	fs::write(&path, toml)
		.await
		.map_err(|e| ConfigError::WriteError { path, source: e })?;

	Ok(())
}

pub async fn load_or_create_config() -> Result<Config, ConfigError> {
	match load_config().await {
		Ok(config) => Ok(config),
		Err(ConfigError::ReadError { path: _, source }) if source.kind() == ErrorKind::NotFound => {
			// if file not found, just create and return a default config
			let config = Config::default();
			save_config(&config).await?;

			Ok(config)
		}
		Err(e) => Err(e),
	}
}
