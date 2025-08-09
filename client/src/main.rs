use std::env;

use anyhow::Result;
use protocol::{
	auth::{LoginRequest, auth_client::AuthClient},
	main::{Empty, main_client::MainClient},
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

	match env::args().nth(1).unwrap().as_str() {
		"login" => {
			let mut client = AuthClient::with_interceptor(channel, add_version);

			let res = client
				.login(LoginRequest {
					username: "test".to_owned(),
				})
				.await?;

			println!("{:?}", res.get_ref());
		}
		token => {
			let mut client = MainClient::with_interceptor(channel, add_version);

			let mut request = Request::new(Empty {});
			request
				.metadata_mut()
				.append("auth", token.parse().unwrap());
			client.say_hello(request).await?;
		}
	}

	Ok(())
}
