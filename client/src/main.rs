use anyhow::Result;
use protocol::{
	Empty,
	main_client::MainClient,
	tonic::{self, Request, transport::Endpoint},
};

fn add_version(mut req: Request<()>) -> tonic::Result<Request<()>> {
	req.metadata_mut()
		.append("version", protocol::VERSION.to_string().parse().unwrap());

	req.metadata_mut().append(
		"auth",
		"550e8400-e29b-41d4-a716-446655440000".parse().unwrap(),
	);

	Ok(req)
}

#[tokio::main]
async fn main() -> Result<()> {
	let channel = Endpoint::from_static("http://[::1]:50051")
		.connect()
		.await?;

	let mut client = MainClient::with_interceptor(channel, add_version);

	client.say_hello(Request::new(Empty {})).await?;

	Ok(())
}
