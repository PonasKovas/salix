use anyhow::Result;
use protocol::{Empty, VERSION, main_client::MainClient, tonic::Request};

#[tokio::main]
async fn main() -> Result<()> {
	let mut client = MainClient::connect("http://[::1]:50051").await?;

	let server_version = client.version_check(Request::new(VERSION)).await?;

	println!("server version={:?}", server_version);

	client.say_hello(Request::new(Empty {})).await?;

	Ok(())
}
