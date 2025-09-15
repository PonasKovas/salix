fn main() {
	build_slint().expect("Slint build failed");
}

fn build_slint() -> Result<(), slint_build::CompileError> {
	slint_build::compile("ui/entry.slint")?;

	Ok(())
}
