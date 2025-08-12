use bitcode::{Decode, Encode};

#[derive(Encode, Decode, Debug)]
pub struct Hello {
	pub message: String,
}
