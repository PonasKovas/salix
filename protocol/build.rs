use std::process::ExitCode;

fn main() -> ExitCode {
	println!("cargo::rerun-if-changed=proto/");

	if let Err(e) = tonic_prost_build::configure()
		.compile_protos(&["proto/main.proto", "proto/auth.proto"], &["proto"])
	{
		eprintln!("Error compiling protobuf schemas:\n{e}");
		return ExitCode::FAILURE;
	}

	ExitCode::SUCCESS
}
