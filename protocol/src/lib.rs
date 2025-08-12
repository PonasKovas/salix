mod enum_macro;
mod message;

use bitcode::{Decode, Encode};
use c2s::*;
use enum_macro::gen_from_impls_for_variants;
use s2c::*;

pub use message::{ReadError, ReadMessage, WriteMessage};

/// Client to server messages
pub mod c2s;
/// Server to client messages
pub mod s2c;

pub const VERSION: u32 = 1;

gen_from_impls_for_variants! {
/// Client to server messages
#[derive(Encode, Decode)]
pub enum C2S {
	Hello(Hello),
}
}

gen_from_impls_for_variants! {
/// Server to client messages
#[derive(Encode, Decode)]
pub enum S2C {

}
}
