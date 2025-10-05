use anyhow::{Context, Result, anyhow};
use client::config::{ClientConfig, load_config};
use std::sync::{Mutex, MutexGuard, OnceLock};

static CONFIG: OnceLock<Mutex<ClientConfig>> = OnceLock::new();

pub fn init() -> Result<()> {
	let config = Mutex::new(load_config()?);

	CONFIG.set(config).map_err(|_| anyhow!("already set"))?;

	Ok(())
}

pub fn config() -> MutexGuard<'static, ClientConfig> {
	CONFIG
		.get()
		.expect("must be initialised first")
		.lock()
		.unwrap()
}
