// Prevent console window in Windows release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result};

mod config;
mod crate_version;
mod entry_window;

slint::include_modules!();

fn main() -> Result<()> {
	config::init().context("reading configuration")?;

	match client::auth::get_stored_auth_token() {
		Ok(Some(token)) => {
			println!("already logged in: {token:?}");
			// try to start the connection, if unauthorized then remove the stored token and show the entry window
			return Ok(());
		}
		Ok(None) => {}
		Err(e) => {
			println!("error fetching stored auth token: {e:?}");
		}
	}

	entry_window::entry_window()?;

	Ok(())
}
