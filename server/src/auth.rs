use protocol::tonic::{Status, server::NamedService};
use sqlx::PgPool;
use std::{
	pin::Pin,
	task::{Context, Poll},
};
use tower::{Layer, Service};
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthData {
	pub user_id: Uuid,
}

pub struct AuthLayer {
	pub db: PgPool,
}

impl<S> Layer<S> for AuthLayer {
	type Service = AuthService<S>;

	fn layer(&self, service: S) -> Self::Service {
		AuthService {
			inner: service,
			db: self.db.clone(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct AuthService<S> {
	inner: S,
	db: PgPool,
}

impl<S, ReqBody, ResBody> Service<http::Request<ReqBody>> for AuthService<S>
where
	S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>> + Clone + Send + 'static,
	S::Future: Send + 'static,
	ReqBody: Send + 'static,
	ResBody: Default,
{
	type Response = S::Response;
	type Error = S::Error;
	type Future = Pin<Box<dyn Future<Output = <S::Future as Future>::Output> + Send>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}
	fn call(&mut self, mut req: http::Request<ReqBody>) -> Self::Future {
		let clone = self.inner.clone();
		let mut inner = std::mem::replace(&mut self.inner, clone);

		let db = self.db.clone();

		Box::pin(async move {
			let auth_token;
			match req.headers().get("auth") {
				Some(token) => {
					let token = match token.to_str() {
						Ok(x) => x,
						Err(_) => {
							return Ok(Status::invalid_argument("`auth` not valid str").into_http());
						}
					};
					let token = match uuid::Uuid::try_parse(token) {
						Ok(x) => x,
						Err(_) => {
							return Ok(
								Status::invalid_argument("`auth` not valid uuid").into_http()
							);
						}
					};

					auth_token = token;
				}
				None => return Ok(Status::permission_denied("no `auth` token").into_http()),
			}

			let user_id = match sqlx::query!(
				r#"SELECT user_id FROM active_sessions WHERE token = $1"#,
				auth_token
			)
			.fetch_optional(&db)
			.await
			{
				Ok(Some(x)) => x.user_id,
				Ok(None) => return Ok(Status::permission_denied("not authorized").into_http()),
				Err(e) => {
					eprintln!("db error: {e}");
					return Ok(Status::internal("database error").into_http());
				}
			};

			req.extensions_mut().insert(AuthData { user_id });

			inner.call(req).await
		})
	}
}

impl<S: NamedService> NamedService for AuthService<S> {
	const NAME: &'static str = S::NAME;
}
