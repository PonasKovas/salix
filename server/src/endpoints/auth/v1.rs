use crate::ServerState;
use anyhow::Context;
use argon2::{
	Algorithm, Argon2, Params, PasswordVerifier, Version,
	password_hash::{SaltString, rand_core::OsRng},
};
use argon2::{PasswordHash, PasswordHasher};
use axum::http::StatusCode;
use axum::{Json, Router, extract::State, response::IntoResponse, routing::post};
use protocol::auth::v1::{self, *};
use sqlx::postgres::types::PgInterval;
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

const AUTH_TOKEN_LIFETIME: PgInterval = PgInterval {
	months: 0,
	days: 15,
	microseconds: 0,
};

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
	let hash = argon2()
		.hash_password(request.password.as_bytes(), &salt)
		.with_context(|| format!("{request:?}"))?
		.to_string();

	let uuid = Uuid::now_v7();

	sqlx::query!(
		r#"INSERT INTO users (id, username, email, password) VALUES ($1, $2, $3, $4)"#,
		uuid,
		request.username,
		request.email,
		hash
	)
	.execute(&state.db)
	.await
	.map_err(|e| {
		if let sqlx::Error::Database(db_err) = e {
			match (db_err.is_unique_violation(), db_err.constraint()) {
				(true, Some("users_username_key")) => Error::UsernameConflict(request.username),
				(true, Some("users_email_key")) => Error::EmailConflict(request.email),
				_ => sqlx::Error::Database(db_err).into(),
			}
		} else {
			e.into()
		}
	})?;

	Ok(StatusCode::OK)
}

async fn login(
	State(state): State<ServerState>,
	Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, Error> {
	let row = sqlx::query!(
		r#"SELECT id, password FROM users WHERE email = $1"#,
		request.email,
	)
	.fetch_one(&state.db)
	.await
	.map_err(|e| {
		if let sqlx::Error::RowNotFound = e {
			Error::Unauthorized
		} else {
			e.into()
		}
	})?;

	let hash = PasswordHash::new(&row.password).with_context(|| format!("{row:?}"))?;

	argon2()
		.verify_password(request.password.as_bytes(), &hash)
		.map_err(|_| Error::Unauthorized)?;

	// great, create a token
	let token = Uuid::now_v7();

	sqlx::query!(
		r#"INSERT INTO active_sessions (token, user_id, expires_at) VALUES ($1, $2, NOW() + $3)"#,
		token,
		row.id,
		AUTH_TOKEN_LIFETIME
	)
	.execute(&state.db)
	.await?;

	Ok(Json(LoginSuccess { auth_token: token }))
}

fn argon2() -> Argon2<'static> {
	Argon2::new(Algorithm::default(), Version::default(), Params::default())
}
