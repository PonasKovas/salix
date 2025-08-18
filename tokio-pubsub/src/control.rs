use super::subscriber::Subscriber;
use tokio::sync::oneshot;

pub(crate) struct CreateSubscriber<T, M> {
	pub response: oneshot::Sender<Subscriber<T, M>>,
}

pub(crate) struct DestroySubscriber {
	pub id: u64,
}

pub(crate) struct SubscribeTopic<T> {
	pub subscriber_id: u64,
	pub topic: T,
}

pub(crate) struct UnsubscribeTopic<T> {
	pub subscriber_id: u64,
	pub topic: T,
}
