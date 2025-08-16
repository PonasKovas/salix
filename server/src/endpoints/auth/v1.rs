use crate::{ServerState, db::user::InsertUserError};
use anyhow::Context;
use argon2::{
	Algorithm, Argon2, Params, PasswordVerifier, Version,
	password_hash::{SaltString, rand_core::OsRng},
};
use argon2::{PasswordHash, PasswordHasher};
use axum::http::StatusCode;
use axum::{Json, Router, extract::State, response::IntoResponse, routing::post};
use protocol::auth::v1::{self, *};
use thiserror::Error;
use tracing::error;

const DEFAULT_AUTH_TOKEN_LIFETIME: u32 = 15 * 24; // in hours

#[derive(Error, Debug)]
pub enum Error {
	#[error("database: {0}")]
	Database(#[from] sqlx::Error),
	#[error(transparent)]
	Internal(#[from] anyhow::Error),
	#[error("username {0} already taken")]
	UsernameConflict(String),
	#[error("email {0} already taken")]
	EmailConflict(String),
	#[error("invalid login")]
	Unauthorized,
}

impl IntoResponse for Error {
	fn into_response(self) -> axum::response::Response {
		error!("{self}");
		match self {
			Error::Database(_) | Error::Internal(_) => {
				(StatusCode::INTERNAL_SERVER_ERROR, Json(v1::Error::Internal))
			}
			Error::UsernameConflict(_) => (StatusCode::CONFLICT, Json(v1::Error::UsernameConflict)),
			Error::EmailConflict(_) => (StatusCode::CONFLICT, Json(v1::Error::EmailConflict)),
			Error::Unauthorized => (StatusCode::UNAUTHORIZED, Json(v1::Error::Unauthorized)),
		}
		.into_response()
	}
}

pub fn routes() -> Router<ServerState> {
	Router::new()
		.route("/login", post(login))
		.route("/new", post(new_account))
}

async fn new_account(
	State(state): State<ServerState>,
	Json(request): Json<NewAccountRequest>,
) -> Result<impl IntoResponse, Error> {
	let salt = SaltString::generate(&mut OsRng);
	let password_hash = argon2()
		.hash_password(request.password.as_bytes(), &salt)
		.with_context(|| format!("{request:?}"))?
		.to_string();

	let _user_id = state
		.db
		.insert_user(&request.username, &request.email, &password_hash)
		.await?
		.map_err(|e| match e {
			InsertUserError::UsernameConflict => Error::UsernameConflict(request.username),
			InsertUserError::EmailConflict => Error::EmailConflict(request.email),
		})?;

	Ok(StatusCode::OK)
}

async fn login(
	State(state): State<ServerState>,
	Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, Error> {
	let user = state
		.db
		.user_by_email(&request.email)
		.await?
		.ok_or(Error::Unauthorized)?;

	let hash = PasswordHash::new(&user.password).with_context(|| format!("{user:?}"))?;

	argon2()
		.verify_password(request.password.as_bytes(), &hash)
		.map_err(|_| Error::Unauthorized)?;

	let token = state
		.db
		.insert_active_sessions(user.id, DEFAULT_AUTH_TOKEN_LIFETIME)
		.await?;

	Ok(Json(LoginSuccess { auth_token: token }))
}

fn argon2() -> Argon2<'static> {
	Argon2::new(Algorithm::default(), Version::default(), Params::default())
}
