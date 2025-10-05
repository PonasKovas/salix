// Prevent console window in Windows release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result, anyhow};
use async_compat::CompatExt;
use client::{Client, auth::AuthToken};
use crate_version::version;
use error_window::show_error_window;

mod chat_window;
mod crate_version;
mod entry_window;
mod error_window;

slint::include_modules!();

fn main() -> Result<()> {
	// since keyring crate is 100% sync we are forced to do this as the very first thing
	let stored_token = client::auth_token_store::get_stored_auth_token()?;

	let loading_window = LoadingWindow::new()?;

	loading_window.set_build_info(version().into());

	let loading_window_weak = loading_window.as_weak();
	slint::spawn_local(
		async move {
			if let Err(e) = init(loading_window_weak.unwrap(), stored_token).await {
				show_error_window(slint::format!("{e}"), true).unwrap();
				println!("{:?}", anyhow!(e));
				return;
			}
		}
		.compat(),
	)?;

	loading_window.show()?;
	slint::run_event_loop()?;

	Ok(())
}

async fn init(loading_window: LoadingWindow, stored_token: Option<AuthToken>) -> Result<()> {
	let config = client::config::load_or_create_config().await?;

	let client = Client::with_config(config);

	if let Some(token) = stored_token {
		client.set_auth_token(token);

		chat_window::chat_window(client)?;
	} else {
		entry_window::entry_window(client.clone())?;
	}

	loading_window.hide()?;

	Ok(())
}
