use crate::{ChatWindow, crate_version::version, error_window::show_error_window};
use async_compat::CompatExt;
use client::{
	Client,
	auth::{self, AuthToken, Error},
};
use slint::{ComponentHandle, ToSharedString};

pub fn chat_window(client: Client) -> anyhow::Result<()> {
	let window = ChatWindow::new()?;

	window.set_build_info(version().into());

	window.show()?;

	window.window().set_maximized(true);

	Ok(())
}
