use ahash::{HashMap, HashMapExt};
use std::{hash::Hash, marker::PhantomData, sync::Arc};
use tokio::{
	select, spawn,
	sync::{
		broadcast::{self, error::RecvError},
		mpsc,
	},
	task::AbortHandle,
};

const MPSC_BUF: usize = 32;
const BROADCAST_BUF: usize = 32;

pub struct Publisher<T, M> {
	next_id: u64,
	subscribers: HashMap<u64, SubscriberData<T, M>>,
	topics: HashMap<T, broadcast::Sender<Arc<M>>>,
}

struct SubscriberData<T, M> {
	mpsc_sender: mpsc::Sender<(T, Message<M>)>,
	topics: HashMap<T, AbortHandle>,
}

pub struct Subscriber<T, M> {
	id: SubscriberId<M>,
	channel: mpsc::Receiver<(T, Message<M>)>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct SubscriberId<M> {
	id: u64,
	phantom: PhantomData<*mut M>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct SubscriberDeleteId<M>(SubscriberId<M>);

pub enum Message<M> {
	Ok(Arc<M>),
	Lagged(u64),
	TopicClosed,
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static, M: Send + Sync + 'static> Publisher<T, M> {
	pub fn new() -> Self {
		Self {
			next_id: 0,
			subscribers: HashMap::new(),
			topics: HashMap::new(),
		}
	}
	pub fn new_subscriber(&mut self) -> Subscriber<T, M> {
		let (sender, receiver) = mpsc::channel(MPSC_BUF);

		let id = self.next_id;
		self.next_id += 1;
		self.subscribers.insert(
			id,
			SubscriberData {
				mpsc_sender: sender,
				topics: HashMap::new(),
			},
		);

		Subscriber {
			id: SubscriberId::new(id),
			channel: receiver,
		}
	}
	pub fn remove_subscriber(&mut self, subscriber_id: SubscriberDeleteId<M>) {
		if let Some(subscriber) = self.subscribers.remove(&subscriber_id.0.id) {
			for (topic, abort_handle) in subscriber.topics {
				abort_handle.abort();
				self.cleanup_topic_if_empty(&topic);
			}
		};
	}
	pub fn subscribe_topic(&mut self, subscriber_id: SubscriberId<M>, topic: T) {
		let subscriber = match self.subscribers.get_mut(&subscriber_id.id) {
			Some(x) => x,
			None => return,
		};
		if let Some(handle) = subscriber.topics.get(&topic) {
			if !handle.is_finished() {
				return;
			}
		}

		let topic_sender = self
			.topics
			.entry(topic.clone())
			.or_insert_with(|| broadcast::Sender::new(BROADCAST_BUF));

		let mpsc_sender = subscriber.mpsc_sender.clone();
		let broadcast_receiver = topic_sender.subscribe();

		let handle =
			spawn(funnel_task(topic.clone(), broadcast_receiver, mpsc_sender)).abort_handle();

		subscriber.topics.insert(topic.clone(), handle);
	}
	pub fn unsubscribe_topic(&mut self, subscriber_id: SubscriberId<M>, topic: &T) {
		let subscriber = match self.subscribers.get_mut(&subscriber_id.id) {
			Some(x) => x,
			None => return,
		};
		if let Some(abort_handle) = subscriber.topics.remove(topic) {
			abort_handle.abort();
			self.cleanup_topic_if_empty(&topic);
		}
	}
	pub fn remove_topic(&mut self, topic: &T) {
		if self.topics.remove(topic).is_some() {
			for subscriber_data in self.subscribers.values_mut() {
				subscriber_data.topics.remove(topic);
			}
		}
	}
	pub fn publish(&mut self, topic: &T, message: M) {
		if let Some(topic_channel) = self.topics.get(topic) {
			// if no listeners thats fine, ignore errors
			let _ = topic_channel.send(Arc::new(message));
		}
	}
	fn cleanup_topic_if_empty(&mut self, topic: &T) {
		if let Some(topic_sender) = self.topics.get(topic) {
			if topic_sender.receiver_count() == 0 {
				self.topics.remove(topic);
			}
		}
	}
}

impl<T, M> Subscriber<T, M> {
	pub fn id(&self) -> SubscriberId<M> {
		self.id
	}
	pub fn delete_id(self) -> SubscriberDeleteId<M> {
		SubscriberDeleteId(self.id)
	}
	/// returns `None` if corresponding publisher is dropped
	///
	/// Otherwise (T, Result)
	/// where T is the topic and Result is the recv result of
	pub async fn recv(&mut self) -> Option<(T, Message<M>)> {
		self.channel.recv().await
	}
}

async fn funnel_task<T: Clone, M>(
	topic: T,
	mut broadcast_receiver: broadcast::Receiver<Arc<M>>,
	mpsc_sender: mpsc::Sender<(T, Message<M>)>,
) {
	loop {
		select! {
			_ = mpsc_sender.closed() => break,
			msg = broadcast_receiver.recv() => {
				let mut closed = false;
				let msg = match msg {
					Ok(x) => Message::Ok(x),
					Err(RecvError::Lagged(n)) => Message::Lagged(n),
					Err(RecvError::Closed) => {
						closed = true;
						Message::TopicClosed
					},
				};

				if mpsc_sender.send((topic.clone(), msg)).await.is_err() {
					break;
				}
				if closed {
					break;
				}
			}
		}
	}
}

impl<M> SubscriberId<M> {
	fn new(id: u64) -> Self {
		Self {
			id,
			phantom: PhantomData,
		}
	}
}
impl<M> Clone for SubscriberId<M> {
	fn clone(&self) -> Self {
		Self {
			id: self.id,
			phantom: PhantomData,
		}
	}
}
impl<M> Copy for SubscriberId<M> {}
