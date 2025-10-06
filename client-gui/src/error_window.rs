use crate::ErrorWindow;
use slint::{ComponentHandle, SharedString};

/// if critical, will end the whole event loop upon clicking OK
pub fn show_error_window(msg: SharedString, critical: bool) -> anyhow::Result<()> {
	let error_window = ErrorWindow::new()?;

	error_window.set_message(msg);

	let error_window_weak = error_window.as_weak();
	error_window.on_ok(move || {
		error_window_weak.unwrap().hide().unwrap();

		if critical {
			slint::quit_event_loop().unwrap();
		}
	});

	error_window.show()?;

	Ok(())
}
