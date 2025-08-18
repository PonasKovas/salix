use super::{
	MpscMessage,
	control::{DestroySubscriber, SubscribeTopic, UnsubscribeTopic},
};
use crate::{Message, Publisher, Topic, error::PublisherDropped, publisher::SubscriberData};
use ahash::{HashMap, HashMapExt};
use tokio::sync::mpsc;

/// A subscriber to a [`Publisher`] instance
///
/// Use this to receive new messages on the topics its subscribed on.
pub struct Subscriber<T, M> {
	id: u64,
	receiver: mpsc::Receiver<MpscMessage<T, M>>,

	subscribe: mpsc::Sender<SubscribeTopic<T>>,
	unsubscribe: mpsc::Sender<UnsubscribeTopic<T>>,
	destroy: mpsc::UnboundedSender<DestroySubscriber>,
}

impl<T, M> Subscriber<T, M> {
	/// Destroys the subscriber
	///
	/// Dropping the [`Subscriber`] has the same effect.
	pub fn destroy(self) {
		// activates Drop impl
	}
	/// Receives a new message from the [`Publisher`] on the subscribed topics
	///
	/// Fails if the [`Publisher`] was dropped.
	pub async fn recv(&mut self) -> Result<MpscMessage<T, M>, PublisherDropped> {
		// this recv() call can return None if all senders are dropped
		// the Publisher has one, as long as the SubscriberData is existing in it's memory.
		// So this can return None if:
		// √ Publisher is dropped as a whole
		// X my SubscriberData is removed from the Publisher, but the only way to do that
		//   is to first drop this Subscriber
		self.receiver.recv().await.ok_or(PublisherDropped)
	}
	/// Subscribes to a topic `T`.
	///
	/// If the publisher side has logic for sending messages on this topic, this
	/// subscriber will start receiving them.
	///
	/// This can be reversed by [`Subscriber::remove_topic`].
	pub async fn add_topic(&mut self, topic: T) -> Result<(), PublisherDropped> {
		self.subscribe
			.send(SubscribeTopic {
				subscriber_id: self.id,
				topic,
			})
			.await
			.map_err(|_| PublisherDropped)
	}
	/// Unsubscribes from a topic `T`.
	pub async fn remove_topic(&mut self, topic: T) -> Result<(), PublisherDropped> {
		self.unsubscribe
			.send(UnsubscribeTopic {
				subscriber_id: self.id,
				topic,
			})
			.await
			.map_err(|_| PublisherDropped)
	}
}

impl<T: Topic, M: Message> Subscriber<T, M> {
	pub(crate) fn new(publisher: &mut Publisher<T, M>) -> Self {
		let (sender, receiver) = mpsc::channel(publisher.options.subscriber_channel_size);

		let id = publisher.next_subscriber_id();
		publisher.subscribers.insert(
			id,
			SubscriberData {
				mpsc_sender: sender,
				funnel_tasks: HashMap::new(),
			},
		);

		Subscriber {
			id,
			receiver,
			subscribe: publisher.subscribe.0.clone(),
			unsubscribe: publisher.unsubscribe.0.clone(),
			destroy: publisher.destroy_subscriber.0.clone(),
		}
	}
}

impl<T, M> Drop for Subscriber<T, M> {
	fn drop(&mut self) {
		// only way this could fail is if the Publisher is dropped,
		// in which case there is nothing to cleanup anymore anyway
		let _ = self.destroy.send(DestroySubscriber { id: self.id });
	}
}
