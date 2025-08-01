fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("capnp/")
        .file("capnp/main.capnp")
        .run()
        .unwrap();
}
