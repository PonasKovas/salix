use crate::{C2S, S2C};
use bitcode::{DecodeOwned, Encode};
use std::pin::pin;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Protocol messages that can be written
pub trait WriteMessage: Sized {
	/// Writes the message to the given `AsyncWrite` object.
	fn write(
		&self,
		to: impl AsyncWrite + Send,
	) -> impl Future<Output = Result<usize, std::io::Error>> + Send;
}
/// Protocol messages that can be read
pub trait ReadMessage: Sized {
	/// Reads the message from the given `AsyncRead` object.
	fn read(from: impl AsyncRead + Send) -> impl Future<Output = Result<Self, Error>> + Send;
}

/// Protocol reading error
#[derive(Error, Debug)]
pub enum Error {
	#[error("io error: {0}")]
	IO(#[from] std::io::Error),
	#[error("{0}")]
	Bitcode(#[from] bitcode::Error),
}

impl WriteMessage for S2C {
	fn write(
		&self,
		to: impl AsyncWrite + Send,
	) -> impl Future<Output = Result<usize, std::io::Error>> + Send {
		encode(self, to)
	}
}
impl ReadMessage for S2C {
	fn read(from: impl AsyncRead + Send) -> impl Future<Output = Result<Self, Error>> + Send {
		decode(from)
	}
}

impl WriteMessage for C2S {
	fn write(
		&self,
		to: impl AsyncWrite + Send,
	) -> impl Future<Output = Result<usize, std::io::Error>> + Send {
		encode(self, to)
	}
}
impl ReadMessage for C2S {
	fn read(from: impl AsyncRead + Send) -> impl Future<Output = Result<Self, Error>> + Send {
		decode(from)
	}
}

pub async fn encode<T: Encode + Sync>(
	val: &T,
	to: impl AsyncWrite + Send,
) -> Result<usize, std::io::Error> {
	let data = bitcode::encode(val);

	let mut to = pin!(to);
	to.write_u32(data.len() as u32).await?;
	to.write_all(&data).await?;

	Ok(4 + data.len())
}

pub async fn decode<T: DecodeOwned>(from: impl AsyncRead + Send) -> Result<T, Error> {
	let mut from = pin!(from);

	let len = from.read_u32().await? as usize;

	let mut buf = vec![0u8; len];
	from.read_exact(&mut buf).await?;

	let parsed = bitcode::decode(&buf)?;

	Ok(parsed)
}
