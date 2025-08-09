use anyhow::Result;
use auth::AuthImpl;
use auth_verify::{AuthData, AuthVerifyLayer};
use protocol::{
	main::{Empty, main_server::Main},
	tonic::{self, Request, Response},
};
use sqlx::PgPool;
use std::env;
use tower::Layer;

mod auth;
mod auth_verify;
mod version_check;

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

#[tokio::main]
async fn main() -> Result<()> {
	let db = PgPool::connect(&env::var("DATABASE_URL")?).await?;

	let auth_layer = AuthVerifyLayer { db: db.clone() };

	tonic::transport::Server::builder()
		.layer(tonic::service::InterceptorLayer::new(
			version_check::version_check,
		))
		.add_service(auth_layer.layer(protocol::main::main_server::MainServer::new(MainImpl)))
		.add_service(protocol::auth::auth_server::AuthServer::new(AuthImpl {
			db: db.clone(),
		}))
		.serve("[::1]:50051".parse()?)
		.await?;

	Ok(())
}
