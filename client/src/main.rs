use futures::AsyncReadExt;
use protocol::capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};
use protocol::main_capnp::handshake;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let msg = std::env::args().nth(1).unwrap().to_string();
	tokio::task::LocalSet::new()
		.run_until(async move {
			let stream = tokio::net::TcpStream::connect("0.0.0.0:15935").await?;
			stream.set_nodelay(true)?;
			let (reader, writer) =
				tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
			let rpc_network = Box::new(twoparty::VatNetwork::new(
				futures::io::BufReader::new(reader),
				futures::io::BufWriter::new(writer),
				rpc_twoparty_capnp::Side::Client,
				Default::default(),
			));
			let mut rpc_system = RpcSystem::new(rpc_network, None);
			let handshake: handshake::Client =
				rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

			tokio::task::spawn_local(rpc_system);

			let mut handshake_request = handshake.handshake_request();
			let mut version = handshake_request.get().init_client_version();
			version.set_major(protocol::VERSION[0]);
			version.set_minor(protocol::VERSION[1]);
			version.set_patch(protocol::VERSION[2]);

			let handshake_reply = handshake_request.send().promise.await?;

			let main = handshake_reply.get()?.get_main()?;
			let server_version = handshake_reply.get()?.get_server_version()?;
			println!(
				"Server version: {major}.{minor}.{patch}",
				major = server_version.get_major(),
				minor = server_version.get_minor(),
				patch = server_version.get_patch(),
			);

			let mut request = main.say_hello_request();
			request.get().init_request().set_name(&msg);

			let reply = request.send().promise.await?;

			println!(
				"received: {}",
				reply.get()?.get_reply()?.get_message()?.to_str()?
			);
			Ok(())
		})
		.await
}
