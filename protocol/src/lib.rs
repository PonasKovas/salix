mod r#macro;
mod message;

use bitcode::{Decode, Encode};
use c2s::*;
use r#macro::{from_variants, message};
use s2c::*;

pub use bitcode::Error;
pub use message::{IntoMessage, Message};

/// Client to server messages
pub mod c2s;
/// Server to client messages
pub mod s2c;

/// `/auth` endpoint
pub mod auth;

pub const VERSION: u32 = 1;

message!(C2S => C2S);
from_variants! {
/// Client to server messages
///
/// This contains all messages that the client can send in the normal state.
/// That doesn't include state specific packets such as [`c2s::Authenticate`] for example.
///
/// Additionally this enum serves as a packet direction marker for the [`Message`] and [`IntoMessage`] traits.
#[derive(Encode, Decode, Debug)]
pub enum C2S {
	SendMessage(SendMessage),
}
}

message!(S2C => S2C);
from_variants! {
/// Server to client messages
///
/// This contains all messages that the server can send in the normal state.
/// That doesn't include state specific packets such as [`s2c::UserInfo`] for example.
///
/// Additionally this enum serves as a packet direction marker for the [`Message`] and [`IntoMessage`] traits.
#[derive(Encode, Decode, Debug)]
pub enum S2C {
	Error(s2c::Error),
	NewMessage(NewMessage),
}
}
