fn main() {
	println!("cargo::rerun-if-changed=capnp/");

	capnpc::CompilerCommand::new()
		.src_prefix("capnp/")
		.file("capnp/main.capnp")
		.run()
		.unwrap();
}
