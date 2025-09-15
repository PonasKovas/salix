slint::include_modules!();

pub fn main_ui() -> anyhow::Result<()> {
	let ui = EntryWindow::new()?;

	ui.run()?;

	Ok(())
}
