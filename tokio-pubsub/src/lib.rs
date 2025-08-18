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
pub use publisher::{Publisher, TopicControl};
pub use publisher_handle::PublisherHandle;
pub use subscriber::Subscriber;
pub use traits::{Message, Topic, TopicContext};

type BroadcastMessage<M> = Arc<M>;
type MpscMessage<T, M> = (T, PubSubMessage<M>);

/// Information when receiving a message
pub enum PubSubMessage<M> {
	/// A new message on the topic
	Ok(Arc<M>),
	/// The subscriber was not reading the messages fast enough
	/// and it missed some messages (in that particular topic, not globally).
	///
	/// The number is how many messages have been skipped.
	Lagged(u64),
}
