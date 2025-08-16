use crate::C2S;
use crate::r#macro::message;
use bitcode::{Decode, Encode};

/// First packet that the client must send to the server
#[derive(Encode, Decode, Debug)]
pub struct Authenticate {
	pub auth_token: [u8; 16],
}
message!(Authenticate => C2S);

#[derive(Encode, Decode, Debug)]
pub struct SendMessage {
	pub message: String,
}
