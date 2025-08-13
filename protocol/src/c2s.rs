use bitcode::{Decode, Encode};

#[derive(Encode, Decode, Debug)]
pub struct Authenticate {
	pub auth_token: [u8; 16],
}

#[derive(Encode, Decode, Debug)]
pub struct SendMessage {
	pub message: String,
}
