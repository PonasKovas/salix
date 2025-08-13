mod enum_macro;
mod message;

use bitcode::{Decode, Encode};
use c2s::*;
use enum_macro::gen_from_impls_for_variants;
use s2c::*;

pub use message::{ReadMessage, WriteMessage};

/// Client to server messages
pub mod c2s;
/// Server to client messages
pub mod s2c;

/// `/auth` endpoint
pub mod auth;

pub const VERSION: u32 = 1;

gen_from_impls_for_variants! {
/// Client to server messages
#[derive(Encode, Decode, Debug)]
pub enum C2S {
	Authenticate(Authenticate),
}
}

gen_from_impls_for_variants! {
/// Server to client messages
#[derive(Encode, Decode, Debug)]
pub enum S2C {
	Error(Error),
	UserInfo(UserInfo),
}
}
