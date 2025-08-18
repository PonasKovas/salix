use crate::{
	Message, Publisher, Topic, control::CreateSubscriber, error::PublisherDropped,
	subscriber::Subscriber,
};
use tokio::sync::{mpsc, oneshot};

/// A handle to a [`Publisher`] instance.
///
/// Cloning this will just give another handle to the same [`Publisher`].
pub struct PublisherHandle<T, M> {
	create_subscriber: mpsc::Sender<CreateSubscriber<T, M>>,
}

impl<T: Topic, M: Message> PublisherHandle<T, M> {
	pub(crate) fn new(publisher: &Publisher<T, M>) -> Self {
		Self {
			create_subscriber: publisher.create_subscriber.0.clone(),
		}
	}
	/// Creates a new [`Subscriber`] to the [`Publisher`].
	pub async fn subscribe(&self) -> Result<Subscriber<T, M>, PublisherDropped> {
		let (sender, receiver) = oneshot::channel();

		self.create_subscriber
			.send(CreateSubscriber { response: sender })
			.await
			.map_err(|_| PublisherDropped)?;

		receiver.await.map_err(|_| PublisherDropped)
	}
}

impl<T: Topic, M: Message> Clone for PublisherHandle<T, M> {
	fn clone(&self) -> Self {
		Self {
			create_subscriber: self.create_subscriber.clone(),
		}
	}
}
