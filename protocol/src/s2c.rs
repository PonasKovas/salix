use bitcode::{Decode, Encode};
use thiserror::Error;

#[derive(Encode, Decode, Debug, Error)]
pub enum Error {
	#[error("invalid packet")]
	InvalidPacket,
	#[error("internal server error")]
	Internal,
}

#[derive(Encode, Decode, Debug)]
pub struct UserInfo {
	pub username: String,
}
