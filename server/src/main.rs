use anyhow::Result;
use protocol::Empty;
use protocol::main_server::Main;
use protocol::tonic;
use protocol::tonic::Request;
use protocol::tonic::Response;
use protocol::tonic::Status;

pub struct MainImpl;

#[tonic::async_trait]
impl Main for MainImpl {
	async fn say_hello(&self, _request: Request<Empty>) -> tonic::Result<Response<Empty>> {
		println!("Someone is saying hello");

		Ok(Response::new(Empty {}))
	}
}

fn version_check(req: Request<()>) -> tonic::Result<Request<()>> {
	match req.metadata().get("version") {
		Some(version) => {
			let version = version
				.to_str()
				.map_err(|_| Status::invalid_argument("`version` not valid str"))?;
			let version = semver::Version::parse(version)
				.map_err(|_| Status::invalid_argument("`version` not valid semver version"))?;

			let requirement = semver::Comparator {
				op: semver::Op::Caret,
				major: protocol::VERSION.major,
				minor: Some(protocol::VERSION.minor),
				patch: Some(protocol::VERSION.patch),
				pre: semver::Prerelease::EMPTY,
			};

			if !requirement.matches(&version) {
				return Err(Status::permission_denied(format!(
					"Incompatible version. Requirement: {requirement}"
				)));
			}

			Ok(req)
		}
		None => Err(Status::permission_denied(
			"Request must contain the `version` metadata",
		)),
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	let addr = "[::1]:50051".parse()?;

	tonic::transport::Server::builder()
		.layer(tonic::service::InterceptorLayer::new(version_check))
		.add_service(protocol::main_server::MainServer::new(MainImpl))
		.serve(addr)
		.await?;

	Ok(())
}
