use bitcode::{Decode, Encode};

#[derive(Encode, Decode, Debug)]
pub enum Error {
	Internal,
}
