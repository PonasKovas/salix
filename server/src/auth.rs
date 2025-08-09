use protocol::auth::auth_server::Auth;
use protocol::auth::{self, LoginFailureReason, LoginRequest, LoginResponse};
use protocol::tonic::{self, Request, Response, Status};
use sqlx::PgPool;
use uuid::Uuid;

pub struct AuthImpl {
	pub db: PgPool,
}

#[tonic::async_trait]
impl Auth for AuthImpl {
	async fn login(
		&self,
		request: Request<LoginRequest>,
	) -> tonic::Result<Response<LoginResponse>> {
		let username = &request.get_ref().username;

		let user_id = match sqlx::query!(r#"SELECT id FROM users WHERE username = $1"#, username)
			.fetch_optional(&self.db)
			.await
		{
			Ok(Some(x)) => x.id,
			Ok(None) => {
				return Ok(Response::new(LoginResponse {
					result: Some(auth::login_response::Result::Failure(auth::LoginFailure {
						code: LoginFailureReason::UserDoesNotExist as i32,
						message: format!("user {username} does not exist"),
					})),
				}));
			}
			Err(e) => {
				eprintln!("database error: {e}");
				return Err(Status::internal("database error"));
			}
		};

		let auth_token = Uuid::now_v7();

		sqlx::query!(
			r#"INSERT INTO active_sessions (token, user_id) VALUES ($1, $2)"#,
			auth_token,
			user_id
		)
		.execute(&self.db)
		.await
		.map_err(|e| {
			eprintln!("database error: {e}");
			Status::internal("database error")
		})?;

		Ok(Response::new(LoginResponse {
			result: Some(auth::login_response::Result::Success(auth::LoginSuccess {
				auth_token: format!("{}", auth_token.as_hyphenated()),
			})),
		}))
	}
}
