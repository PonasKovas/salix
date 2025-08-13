use serde::{Deserialize, Serialize};
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
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "code")]
pub enum Error {
	Internal,
	Unauthorized,
	UsernameConflict,
	EmailConflict,
}
