use bitcode::{Decode, Encode};

#[derive(Encode, Decode)]
pub struct Pong;

#[derive(Encode, Decode)]
pub struct Hello {
	pub message: String,
}
