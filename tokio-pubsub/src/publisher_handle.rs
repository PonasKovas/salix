use crate::{
	Message, Topic, TopicContext, control::ControlMessage, error::PublisherDropped,
	subscriber::Subscriber,
};
use tokio::sync::{mpsc, oneshot};

/// A handle to a [`Publisher`] instance.
///
/// To be able to use it, the main [`Publisher`] instance must be driven ([`Publisher::drive`]),
/// Otherwise all calls will hang indefinitely
///
/// Cloning this will just give another handle to the same [`Publisher`].
pub struct PublisherHandle<T: Topic, M: Message, C: TopicContext> {
	control: mpsc::Sender<ControlMessage<T, M, C>>,
}

impl<T: Topic, M: Message, C: TopicContext> PublisherHandle<T, M, C> {
	pub(crate) fn new(control: mpsc::Sender<ControlMessage<T, M, C>>) -> Self {
		Self { control }
	}
	/// Creates a new [`Subscriber`] to the [`Publisher`].
	pub async fn subscribe(&self) -> Result<Subscriber<T, M, C>, PublisherDropped> {
		let (sender, receiver) = oneshot::channel();

		self.control
			.send(ControlMessage::CreateSubscriber { response: sender })
			.await
			.map_err(|_| PublisherDropped)?;

		receiver.await.map_err(|_| PublisherDropped)
	}
}

impl<T: Topic, M: Message, C: TopicContext> Clone for PublisherHandle<T, M, C> {
	fn clone(&self) -> Self {
		Self {
			control: self.control.clone(),
		}
	}
}
