use super::{
	MPSC_BUF_SIZE, MpscMessage, Publisher, SubscriberData,
	control::{DestroySubscriber, SubscribeTopic, UnsubscribeTopic},
};
use ahash::{HashMap, HashMapExt};
use std::hash::Hash;
use tokio::sync::mpsc;

pub struct Subscriber<T, M> {
	id: u64,
	receiver: mpsc::Receiver<MpscMessage<T, M>>,

	subscribe: mpsc::Sender<SubscribeTopic<T>>,
	unsubscribe: mpsc::Sender<UnsubscribeTopic<T>>,
	destroy: mpsc::UnboundedSender<DestroySubscriber>,
}

impl<T, M> Subscriber<T, M> {
	pub fn destroy(self) {
		// activates Drop impl
	}
	pub async fn recv(&mut self) -> MpscMessage<T, M> {
		// this recv() call can return None if all senders are dropped
		// the Publisher has one, as long as the SubscriberData is existing in it's memory.
		// So this can return None if:
		// √ Publisher is dropped as a whole
		// X my SubscriberData is removed from the Publisher, but the only way to do that
		//   is to first drop this Subscriber
		self.receiver.recv().await.unwrap()
	}
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static, M: Send + Sync + 'static> Subscriber<T, M> {
	pub(super) fn new(publisher: &mut Publisher<T, M>) -> Self {
		let (sender, receiver) = mpsc::channel(MPSC_BUF_SIZE);

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
		// only way this could fail is if the Publisher is dropped, in which case there is nothing to cleanup anymore
		let _ = self.destroy.send(DestroySubscriber { id: self.id });
	}
}
