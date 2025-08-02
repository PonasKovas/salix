use anyhow::{Context, Result};
use futures::AsyncReadExt;
use protocol::capnp::{self, capability::Promise};
use protocol::capnp_rpc::{self, RpcSystem, rpc_twoparty_capnp, twoparty};
use protocol::main_capnp::{handshake, main};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::LocalSet;
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::{
	TlsAcceptor,
	rustls::{
		self,
		pki_types::{CertificateDer, PrivateKeyDer},
	},
};

struct HandshakeImpl;

impl handshake::Server for HandshakeImpl {
	fn handshake(
		&mut self,
		params: handshake::HandshakeParams,
		mut results: handshake::HandshakeResults,
	) -> Promise<(), capnp::Error> {
		let client_version = params.get()?.get_client_version()?;
		println!(
			"Client version: {major}.{minor}.{patch}",
			major = client_version.get_major(),
			minor = client_version.get_minor(),
			patch = client_version.get_patch(),
		);

		let mut server_version = results.get().init_server_version();
		server_version.set_major(protocol::VERSION[0]);
		server_version.set_minor(protocol::VERSION[1]);
		server_version.set_patch(protocol::VERSION[2]);

		results.get().set_main(capnp_rpc::new_client(MainImpl));

		Promise::ok(())
	}
}

struct MainImpl;

impl main::Server for MainImpl {
	fn say_hello(
		&mut self,
		params: main::SayHelloParams,
		mut results: main::SayHelloResults,
	) -> Promise<(), capnp::Error> {
		let request = params.get()?.get_request()?;
		let name = request.get_name()?.to_str().map_err(Into::into)?;
		let message = format!("Hello, {name}!");
		results.get().init_reply().set_message(message);

		Promise::ok(())
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	let certs =
		CertificateDer::pem_file_iter("certs/server.crt")?.collect::<Result<Vec<_>, _>>()?;
	let key = PrivateKeyDer::from_pem_file("certs/server.key")?;

	let config = rustls::ServerConfig::builder()
		.with_no_client_auth()
		.with_single_cert(certs, key)?;
	let tls_acceptor = TlsAcceptor::from(Arc::new(config));

	let listener = TcpListener::bind("0.0.0.0:15935").await?;

	let handshake_client: handshake::Client = capnp_rpc::new_client(HandshakeImpl);

	LocalSet::new()
		.run_until(async move {
			loop {
				let (stream, _) = listener.accept().await?;
				stream.set_nodelay(true)?;

				let tls_acceptor = tls_acceptor.clone();
				let handshake_client = handshake_client.clone();

				tokio::task::spawn_local(async move {
					if let Err(e) = accept_con(tls_acceptor, handshake_client, stream).await {
						eprintln!("connection error: {e:?}");
					}
				});
			}
		})
		.await
}

async fn accept_con(
	tls_acceptor: TlsAcceptor,
	handshake_client: handshake::Client,
	stream: TcpStream,
) -> Result<()> {
	let stream = tls_acceptor
        .accept(stream).await.context("tls initialization")?;

	let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
	let network = twoparty::VatNetwork::new(
		futures::io::BufReader::new(reader),
		futures::io::BufWriter::new(writer),
		rpc_twoparty_capnp::Side::Server,
		Default::default(),
	);

	let rpc_system = RpcSystem::new(Box::new(network), Some(handshake_client.client));

	rpc_system.await?;

	Ok(())
}
