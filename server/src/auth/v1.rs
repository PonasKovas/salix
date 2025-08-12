use crate::ServerState;
use argon2::{
	Algorithm, Argon2, Params, PasswordVerifier, Version,
	password_hash::{SaltString, rand_core::OsRng},
};
use argon2::{PasswordHash, PasswordHasher};
use axum::http::StatusCode;
use axum::{Json, Router, extract::State, response::IntoResponse, routing::post};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

pub fn routes() -> Router<ServerState> {
	Router::new()
		.route("/login", post(login))
		.route("/new", post(new_account))
}

#[derive(Deserialize)]
struct NewAccountRequest {
	username: String,
	email: String,
	password: String,
}

#[derive(Deserialize)]
struct LoginRequest {
	email: String,
	password: String,
}

async fn new_account(
	State(state): State<ServerState>,
	Json(request): Json<NewAccountRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
	let argon2 = Argon2::new(Algorithm::default(), Version::default(), Params::default());
	let salt = SaltString::generate(&mut OsRng);

	let hash = argon2
		.hash_password(request.password.as_bytes(), &salt)
		.map_err(|e| {
			error!("Error hashing password: {e}");
			(StatusCode::INTERNAL_SERVER_ERROR, format!("internal error"))
		})?
		.to_string();

	let uuid = Uuid::now_v7();

	match sqlx::query!(
		r#"INSERT INTO users (id, username, email, password) VALUES ($1, $2, $3, $4)"#,
		uuid,
		request.username,
		request.email,
		hash
	)
	.execute(&state.db)
	.await
	{
		Ok(_) => {}
		Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
			match db_err.constraint() {
				Some("username") => {
					return Err((
						StatusCode::CONFLICT,
						format!("username {} is already taken", request.username),
					));
				}
				Some("email") => {
					return Err((
						StatusCode::CONFLICT,
						format!("email {} is already taken", request.email),
					));
				}
				other => {
					error!("db unique violation {other:?}");
					return Err((StatusCode::CONFLICT, format!("already taken")));
				}
			}
		}
		Err(e) => {
			error!("db error: {e}");
			return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("internal error")));
		}
	}

	Ok(StatusCode::OK)
}

async fn login(
	State(state): State<ServerState>,
	Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
	let unauthorized_err = Err((StatusCode::UNAUTHORIZED, "invalid login"));

	let (uuid, password_hash) = match sqlx::query!(
		r#"SELECT id, password FROM users WHERE email = $1"#,
		request.email,
	)
	.fetch_one(&state.db)
	.await
	{
		Ok(x) => (x.id, x.password),
		Err(sqlx::Error::RowNotFound) => {
			return unauthorized_err;
		}
		Err(e) => {
			error!("db error: {e}");
			return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal error"));
		}
	};

	let argon2 = Argon2::new(Algorithm::default(), Version::default(), Params::default());

	let hash = match PasswordHash::new(&password_hash) {
		Ok(x) => x,
		Err(e) => {
			error!("db stored password hash {password_hash:?} not valid hash: {e}");
			return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal error"));
		}
	};

	let correct = argon2
		.verify_password(request.password.as_bytes(), &hash)
		.is_ok();

	if !correct {
		return unauthorized_err;
	}

	Ok("ok")
}
