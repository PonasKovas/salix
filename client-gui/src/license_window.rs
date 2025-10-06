use crate::LicenseWindow;
use slint::{ComponentHandle, SharedString};

pub fn show_license_window() -> anyhow::Result<()> {
	let license_window = LicenseWindow::new()?;

	license_window.set_license_text("TEST".into());

	println!("TEDST");

	license_window.show()?;

	Ok(())
}
