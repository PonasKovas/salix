use std::convert::Infallible;

use crate::{
	Message, Topic, TopicContext, control::ControlMessage, error::PublisherDropped,
	subscriber::Subscriber, traits::TopicError,
};
use tokio::sync::{mpsc, oneshot};

/// A handle to a [`Publisher`][crate::Publisher] instance.
///
/// To be able to use it, the main [`Publisher`][crate::Publisher] instance must be driven ([`Publisher::drive`][crate::Publisher::drive]),
/// Otherwise all calls will hang indefinitely
///
/// Cloning this will just give another handle to the same [`Publisher`][crate::Publisher].
pub struct PublisherHandle<T: Topic, M: Message, C: TopicContext, E: TopicError = Infallible> {
	control: mpsc::Sender<ControlMessage<T, M, C, E>>,
}

impl<T: Topic, M: Message, C: TopicContext, E: TopicError> PublisherHandle<T, M, C, E> {
	pub(crate) fn new(control: mpsc::Sender<ControlMessage<T, M, C, E>>) -> Self {
		Self { control }
	}
	/// Creates a new [`Subscriber`] to the [`Publisher`][crate::Publisher].
	pub async fn subscribe(&self) -> Result<Subscriber<T, M, C, E>, PublisherDropped> {
		let (sender, receiver) = oneshot::channel();

		self.control
			.send(ControlMessage::CreateSubscriber { response: sender })
			.await
			.map_err(|_| PublisherDropped)?;

		receiver.await.map_err(|_| PublisherDropped)
	}
}

impl<T: Topic, M: Message, C: TopicContext, E: TopicError> Clone for PublisherHandle<T, M, C, E> {
	fn clone(&self) -> Self {
		Self {
			control: self.control.clone(),
		}
	}
}
