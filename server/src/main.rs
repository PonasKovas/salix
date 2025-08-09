use std::env;

use anyhow::Result;
use auth::AuthData;
use auth::AuthLayer;
use protocol::Empty;
use protocol::main_server::Main;
use protocol::tonic;
use protocol::tonic::Request;
use protocol::tonic::Response;
use protocol::tonic::Status;
use sqlx::PgPool;
use tower::Layer;

mod auth;

pub struct MainImpl;

#[tonic::async_trait]
impl Main for MainImpl {
	async fn say_hello(&self, request: Request<Empty>) -> tonic::Result<Response<Empty>> {
		println!(
			"{:?} is saying hello",
			request.extensions().get::<AuthData>().unwrap().user_id
		);

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
	let db = PgPool::connect(&env::var("DATABASE_URL")?).await?;

	let auth_layer = AuthLayer { db: db.clone() };

	tonic::transport::Server::builder()
		.layer(tonic::service::InterceptorLayer::new(version_check))
		.add_service(auth_layer.layer(protocol::main_server::MainServer::new(MainImpl)))
		.serve("[::1]:50051".parse()?)
		.await?;

	Ok(())
}
