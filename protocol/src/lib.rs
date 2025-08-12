mod enum_macro;
mod message;

use bitcode::{Decode, Encode};
use c2s::*;
use enum_macro::gen_from_impls_for_variants;
use s2c::*;
use tokio::io::AsyncWrite;

pub use message::{Error, ReadMessage, WriteMessage};

/// Client to server messages
pub mod c2s;
/// Server to client messages
pub mod s2c;

gen_from_impls_for_variants! {
/// Client to server messages
#[derive(Encode, Decode)]
pub enum C2S {
	Pong(Pong),
}
}

gen_from_impls_for_variants! {
/// Server to client messages
#[derive(Encode, Decode)]
pub enum S2C {
	Ping(Ping),
}
}
