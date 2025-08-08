use anyhow::Result;
use protocol::Empty;
use protocol::VERSION;
use protocol::Version;
use protocol::main_server::Main;
use protocol::tonic;
use protocol::tonic::Request;
use protocol::tonic::Response;

pub struct MainImpl;

#[tonic::async_trait]
impl Main for MainImpl {
	async fn version_check(&self, request: Request<Version>) -> tonic::Result<Response<Version>> {
		let client_version = request.get_ref();
		println!(
			"Client version: {}.{}.{}",
			client_version.major, client_version.minor, client_version.patch,
		);

		// send back our own version
		Ok(Response::new(VERSION))
	}
	async fn say_hello(&self, _request: Request<Empty>) -> tonic::Result<Response<Empty>> {
		println!("Someone is saying hello");

		Ok(Response::new(Empty {}))
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	let addr = "[::1]:50051".parse()?;

	tonic::transport::Server::builder()
		.add_service(protocol::main_server::MainServer::new(MainImpl))
		.serve(addr)
		.await?;

	Ok(())
}
