use crate::{C2S, S2C};
use bitcode::{DecodeOwned, Encode};
use thiserror::Error;

/// Protocol messages that can be written
pub trait WriteMessage {
	/// Writes the message to the given `AsyncWrite` object.
	fn write(&self) -> Vec<u8>;
}
/// Protocol messages that can be read
pub trait ReadMessage: Sized {
	/// Reads the message from the given `AsyncRead` object.
	fn read(from: &[u8]) -> Result<Self, ReadError>;
}

/// Protocol reading error
#[derive(Error, Debug)]
#[error("{0}")]
pub struct ReadError(#[from] bitcode::Error);

impl WriteMessage for S2C {
	fn write(&self) -> Vec<u8> {
		encode(self)
	}
}
impl ReadMessage for S2C {
	fn read(from: &[u8]) -> Result<Self, ReadError> {
		decode(from)
	}
}

impl WriteMessage for C2S {
	fn write(&self) -> Vec<u8> {
		encode(self)
	}
}
impl ReadMessage for C2S {
	fn read(from: &[u8]) -> Result<Self, ReadError> {
		decode(from)
	}
}

fn encode<T: Encode>(val: &T) -> Vec<u8> {
	bitcode::encode(val)
}

fn decode<T: DecodeOwned>(from: &[u8]) -> Result<T, ReadError> {
	Ok(bitcode::decode(from)?)
}
