use crate::{ChatWindow, crate_version::version, error_window::show_error_window};
use async_compat::CompatExt;
use client::{
	Client,
	auth::{self, AuthToken, Error},
};
use slint::{ComponentHandle, ToSharedString};

pub fn chat_window(client: Client) -> anyhow::Result<()> {
	let window = ChatWindow::new()?;

	let global_data: crate::GlobalData = window.global();
	global_data.set_build_info(version().into());
	global_data.on_show_license(move || {
		if let Err(e) = crate::license_window::show_license_window() {
			println!("{:?}", anyhow::anyhow!(e));
		}
	});

	window.show()?;

	window.window().set_maximized(true);

	Ok(())
}
