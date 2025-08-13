use bitcode::{Decode, Encode};
use thiserror::Error;

#[derive(Encode, Decode, Debug, Error)]
pub enum Error {
	#[error("invalid packet")]
	InvalidPacket,
	#[error("internal server error")]
	Internal,
	#[error("unauthenticated")]
	Unauthenticated,
	#[error("invalid auth token")]
	Unauthorized,
	#[error("unexpected packet")]
	UnexpectedPacket,
}

#[derive(Encode, Decode, Debug)]
pub struct UserInfo {
	pub username: String,
}

#[derive(Encode, Decode, Debug)]
pub struct NewMessage {
	pub user: String,
	pub message: String,
}
