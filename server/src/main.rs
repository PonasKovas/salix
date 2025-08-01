use std::time::Duration;

use anyhow::Result;
use futures::AsyncReadExt;
use protocol::capnp::{self, capability::Promise};
use protocol::capnp_rpc::{self, RpcSystem, rpc_twoparty_capnp, twoparty};
use protocol::main_capnp::{handshake, main};
use tokio::net::TcpListener;
use tokio::spawn;

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
        server_version.set_major(1);
        server_version.set_minor(2);
        server_version.set_patch(3);

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
    let http2 = hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
        .keep_alive_interval(Duration::from_secs(5));

    let listener = TcpListener::bind("0.0.0.0:15935").await?;

    loop {
        let (stream, _) = listener.accept().await?;
        stream.set_nodelay(true)?;

        let stream = hyper_util::rt::TokioIo::new(stream);

        spawn(http2.serve_connection(
            stream,
            hyper::service::service_fn(|request| Ok(hyper::Response)),
        ));
    }

    // tokio::task::LocalSet::new()
    //     .run_until(async move {
    //         let handshake_client: handshake::Client = capnp_rpc::new_client(HandshakeImpl);

    //         loop {
    //             let (stream, _) = listener.accept().await?;
    //             stream.set_nodelay(true)?;
    //             let (reader, writer) =
    //                 tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
    //             let network = twoparty::VatNetwork::new(
    //                 futures::io::BufReader::new(reader),
    //                 futures::io::BufWriter::new(writer),
    //                 rpc_twoparty_capnp::Side::Server,
    //                 Default::default(),
    //             );

    //             let rpc_system =
    //                 RpcSystem::new(Box::new(network), Some(handshake_client.clone().client));

    //             tokio::task::spawn_local(rpc_system);
    //         }
    //     })
    //     .await
}
