use slint_build::CompilerConfiguration;
use std::{error::Error, process::Stdio};

fn main() {
	println!(
		"cargo:rustc-env=GIT_COMMIT_HASH={}",
		get_commit_hash().expect("failed to get commit hash")
	);

	build_slint().expect("Slint build failed");
}

fn get_commit_hash() -> Result<String, Box<dyn Error>> {
	let output = std::process::Command::new("git")
		.args(["rev-parse", "--short", "HEAD"])
		.stderr(Stdio::inherit())
		.output()?;

	if output.status.success() {
		let stdout = String::from_utf8(output.stdout)?;

		Ok(stdout.trim().to_string())
	} else {
		Err(format!("git command returned non-zero status: {}", output.status).into())
	}
}

fn build_slint() -> Result<(), slint_build::CompileError> {
	let config = CompilerConfiguration::default().with_style("cupertino-dark".to_owned());
	slint_build::compile_with_config("ui/main.slint", config)?;

	Ok(())
}
