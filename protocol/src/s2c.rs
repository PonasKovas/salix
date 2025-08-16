use crate::S2C;
use crate::r#macro::message;
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
	#[error("timed out")]
	TimedOut,
	#[error("unexpected text frame")]
	TextFrame,
}

#[derive(Encode, Decode, Debug)]
pub struct UserInfo {
	pub username: String,
}
message!(UserInfo => S2C);

#[derive(Encode, Decode, Debug)]
pub struct NewMessage {
	pub user: String,
	pub message: String,
}
