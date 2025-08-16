use bitcode::{DecodeOwned, Encode};

/// Protocol messages
pub trait Message<D>: Sized {
	fn write(&self) -> Vec<u8>;
	fn read(from: &[u8]) -> Result<Self, crate::Error>;
}

pub(crate) fn encode<T: Encode>(val: &T) -> Vec<u8> {
	bitcode::encode(val)
}

pub(crate) fn decode<T: DecodeOwned>(from: &[u8]) -> Result<T, crate::Error> {
	Ok(bitcode::decode(from)?)
}

// I would simply use `impl Into<P> where P: Message<D>` but compiler cant infer types then
/// Can be converted into a message
pub trait IntoMessage<D> {
	fn into_message(self) -> impl Message<D>;
}
impl<T: Message<D>, D> IntoMessage<D> for T {
	fn into_message(self) -> impl Message<D> {
		self
	}
}
