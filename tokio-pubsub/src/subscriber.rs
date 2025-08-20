use super::MpscMessage;
use crate::{
	Message, Topic, TopicContext,
	control::ControlMessage,
	error::{AddTopicError, PublisherDropped, RemoveTopicError},
	traits::TopicError,
};
use std::{convert::Infallible, mem::forget};
use tokio::sync::{mpsc, oneshot};

/// A subscriber to a [`Publisher`][crate::Publisher] instance
///
/// Use this to receive new messages on the topics its subscribed on.
///
/// To be able to use the control methods (adding/removing topics, destroying the subscriber),
/// the main [`Publisher`][crate::Publisher] instance must be driven ([`Publisher::drive`][crate::Publisher::drive]),
/// Otherwise these calls will hang indefinitely.
pub struct Subscriber<T: Topic, M: Message, C: TopicContext, E: TopicError = Infallible> {
	id: u64,
	receiver: mpsc::Receiver<MpscMessage<T, M>>,

	// Option only to allow to move it out in the Drop impl
	control: Option<mpsc::Sender<ControlMessage<T, M, C, E>>>,
}

impl<T: Topic, M: Message, C: TopicContext, E: TopicError> Subscriber<T, M, C, E> {
	/// Destroys the subscriber
	///
	/// Dropping the [`Subscriber`] has the same effect.
	pub async fn destroy(self) {
		// it will error if Publisher is dropped, in which case there is nothing
		// to clean up anymore anyway so its fine
		let _ = self
			.control()
			.send(ControlMessage::DestroySubscriber { id: self.id })
			.await;

		// avoid running the drop impl which would do the same but spawn a task for it
		forget(self);
	}
	/// Receives a new message from the [`Publisher`][crate::Publisher] on the subscribed topics
	///
	/// Fails if the [`Publisher`][crate::Publisher] was dropped.
	pub async fn recv(&mut self) -> Result<MpscMessage<T, M>, PublisherDropped> {
		// this recv() call can return None if all senders are dropped
		// the Publisher has one, as long as the SubscriberData is existing in it's memory.
		// So this can return None if:
		// X my SubscriberData is removed from the Publisher, but the only way to do that
		//   is to first drop this Subscriber
		// √ Publisher is dropped as a whole
		self.receiver.recv().await.ok_or(PublisherDropped)
	}
	/// Subscribes to a topic `T`.
	///
	/// If the publisher side has logic for sending messages on this topic, this
	/// subscriber will start receiving them.
	///
	/// This can be reversed by [`Subscriber::remove_topic`].
	pub async fn add_topic(&mut self, topic: T) -> Result<C, AddTopicError<E>> {
		let (response_sender, response_receiver) = oneshot::channel();

		self.control()
			.send(ControlMessage::AddTopic {
				id: self.id,
				topic,
				response: response_sender,
			})
			.await
			.map_err(|_| PublisherDropped)?;

		Ok(response_receiver
			.await
			.map_err(|_| PublisherDropped)??
			.map_err(AddTopicError::TopicError)?)
	}
	/// Unsubscribes from a topic `T`. Nothing will happen if the topic was not subscribed
	pub async fn remove_topic(&mut self, topic: T) -> Result<(), RemoveTopicError> {
		let (response_sender, response_receiver) = oneshot::channel();

		self.control()
			.send(ControlMessage::RemoveTopic {
				id: self.id,
				topic,
				response: response_sender,
			})
			.await
			.map_err(|_| PublisherDropped)?;

		Ok(response_receiver.await.map_err(|_| PublisherDropped)??)
	}
}

impl<T: Topic, M: Message, C: TopicContext, E: TopicError> Subscriber<T, M, C, E> {
	pub(crate) fn new(
		id: u64,
		control: mpsc::Sender<ControlMessage<T, M, C, E>>,
		receiver: mpsc::Receiver<MpscMessage<T, M>>,
	) -> Self {
		Subscriber {
			id,
			receiver,
			control: Some(control),
		}
	}
	fn control(&self) -> &mpsc::Sender<ControlMessage<T, M, C, E>> {
		self.control.as_ref().unwrap()
	}
}

/// Prefer using the [`Subscriber::destroy`] method instead of dropping, because dropping will spawn a task
impl<T: Topic, M: Message, C: TopicContext, E: TopicError> Drop for Subscriber<T, M, C, E> {
	fn drop(&mut self) {
		let control = self.control.take().unwrap();
		let id = self.id;

		tokio::runtime::Handle::current().spawn(async move {
			// only way this could fail is if the Publisher is dropped,
			// in which case there is nothing to cleanup anymore anyway
			let _ = control.send(ControlMessage::DestroySubscriber { id }).await;
		});
	}
}
