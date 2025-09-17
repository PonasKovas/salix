use crate::crate_version::version;

slint::include_modules!();

pub fn entry_window() -> anyhow::Result<()> {
	let entry = EntryWindow::new()?;

	entry.set_build_info(version().into());
	entry.on_login(|username, password| {
		println!("LOGIN: {}:{}", username, password);
	});

	entry.run()?;

	Ok(())
}
