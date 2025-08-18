use super::subscriber::Subscriber;
use tokio::sync::oneshot;

pub struct CreateSubscriber<T, M> {
	pub response: oneshot::Sender<Subscriber<T, M>>,
}

pub struct DestroySubscriber {
	pub id: u64,
}

pub struct SubscribeTopic<T> {
	pub subscriber_id: u64,
	pub topic: T,
}

pub struct UnsubscribeTopic<T> {
	pub subscriber_id: u64,
	pub topic: T,
}
