//! A local Publisher/Subscriber system built on tokio broadcast and mpsc channels.
//!
//! ![Diagram](https://raw.githubusercontent.com/PonasKovas/salix/refs/heads/master/tokio-pubsub/diagram.svg)
//!
//! # Guide
//!
//! - Create a new [`Publisher`]
//!   - `T` parameter - the topic type. It can be anything, like a chatroom ID, username, etc.
//!     Beware that it will be cloned a lot, so prefer using cheap types such as integers instead of strings,
//!     but if you need to use a string, you can put it inside an [`Arc`].
//!   - `M` parameter - the message type. This will be automatically put in an [`Arc`] and never actually cloned.
//!     All subscribers to a topic will receive a reference to a message sent in that topic.
//!   - `C` parameter - the topic context type. This will be returned to subscribers when they subscribe to a new
//!     topic, can be left `()` if you don't need this. Some examples: use `Result` if only specific topics can be subscribed to,
//!     to indicate an invalid topic to the subscriber, or send some topic state, like the last sent message ID in a chatroom.
//! - Make a handle to the publisher - [`PublisherHandle`]
//!   - The handle can be cheaply cloned and will point to the same [`Publisher`]
//!   - It can be used to create new [`Subscriber`]
//! - Create a [`Subscriber`] using the handle or with the [`Publisher`] directly
//! - Add/remove topics on the subscriber that you are interested to receive messages on.
//! - Use [`Subscriber::recv`] to receive messages `(T, Arc<M>)` where `T` is the topic id, and `M` is the message.
//! - Drive the [`Publisher`] in a loop with [`Publisher::drive`] to keep it functioning.
//! - Use [`Publisher::publish`] to publish new messages on a specific topic. This requires mutable access to the publisher
//!   so naturally you will have one task publishing and multiple tasks receiving. However, if you wish to publish from different
//!   tasks you can easily do that by creating a new mpsc channel just having a task that receives from it and publishes all messages
//!   (but dont forget to also [`drive`][Publisher::drive] the [`Publisher`])
//!
//! # Lagging
//!
//! As this crate uses tokio broadcast channels for topic-specific messages, it has the same lagging
//! functionality as the [broadcast][tokio::sync::broadcast] channel.
//! If you don't want your subscribers to miss any messages you must take this into account.

use std::sync::Arc;

mod control;
/// Error types
pub mod error;
mod funnel_task;
mod options;
mod publisher;
mod publisher_handle;
mod subscriber;
mod traits;

pub use options::Options;
pub use publisher::{EventReactor, Publisher, PublisherDriver};
pub use publisher_handle::PublisherHandle;
pub use subscriber::Subscriber;
pub use traits::{Message, Topic, TopicContext, TopicError};

type BroadcastMessage<M> = Arc<M>;
type MpscMessage<T, M> = (T, PubSubMessage<M>);

/// Information when receiving a message
#[derive(Debug)]
pub enum PubSubMessage<M> {
	/// A new message on the topic
	Ok(Arc<M>),
	/// The subscriber was not reading the messages fast enough
	/// and it missed some messages (in that particular topic, not globally).
	///
	/// The number is how many messages have been skipped.
	Lagged(u64),
}
