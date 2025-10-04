//! v1 of the auth API
//!
//! All types that implement [`Request`] to be sent as JSON in HTTP POST request
//! to their respective path

use super::Request;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

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
	#[error("incorrect code")]
	IncorrectCode,
	/// the client is responsible for validating requirements such as
	/// the email being valid, or the username being long enough, etc
	#[error("invalid request")]
	InvalidRequest,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginRequest {
	pub email: String,
	pub password: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct LoginSuccess {
	pub auth_token: Uuid,
}
impl Request for LoginRequest {
	type Response = LoginSuccess;
	type Error = Error;

	const PATH: &'static str = "/login";
}

/// First stage when creating a new account
#[derive(Serialize, Deserialize, Debug)]
pub struct StartEmailVerifyRequest {
	/// must be a valid email
	pub email: String,
}
impl Request for StartEmailVerifyRequest {
	type Response = ();
	type Error = Error;

	const PATH: &'static str = "/start_email_verify";
}

/// Second stage when creating a new account
#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyEmailRequest {
	pub email: String,
	pub code: u32,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyEmailResponse {
	pub registration_id: Uuid,
}
impl Request for VerifyEmailRequest {
	type Response = VerifyEmailResponse;
	type Error = Error;

	const PATH: &'static str = "/verify_email";
}

/// Third stage when creating a new account
#[derive(Serialize, Deserialize, Debug)]
pub struct FinalizeNewAccountRequest {
	pub registration_id: Uuid,
	pub username: String,
	pub password: String,
}
impl Request for FinalizeNewAccountRequest {
	type Response = LoginSuccess;
	type Error = Error;

	const PATH: &'static str = "/finalize_new_account";
}
