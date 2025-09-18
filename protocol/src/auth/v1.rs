use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// `POST /auth/v1/new` **request** JSON payload
#[derive(Serialize, Deserialize, Debug)]
pub struct NewAccountRequest {
	pub username: String,
	pub email: String,
	pub password: String,
}

/// `POST /auth/v1/login` **request** JSON payload
#[derive(Serialize, Deserialize, Debug)]
pub struct LoginRequest {
	pub email: String,
	pub password: String,
}

/// `POST /auth/v1/login` **response** JSON payload
#[derive(Serialize, Deserialize, Debug)]
pub struct LoginSuccess {
	pub auth_token: Uuid,
}

/// All possible errors that can be returned in `/auth/v1` as JSON
#[derive(Serialize, Deserialize, Debug, Error)]
#[serde(tag = "code")]
pub enum Error {
	#[error("internal server error")]
	Internal,
	#[error("unauthorized")]
	Unauthorized,
	#[error("username taken")]
	UsernameConflict,
	#[error("email already registered")]
	EmailConflict,
	/// the client is responsible for validating requirements such as
	/// the email being valid, or the username being long enough, etc
	#[error("invalid request")]
	InvalidRequest,
}
