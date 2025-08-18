use ahash::{HashMap, HashMapExt};
use control::{CreateSubscriber, DestroySubscriber, SubscribeTopic, UnsubscribeTopic};
use std::{collections::hash_map::Entry, hash::Hash, sync::Arc};
use subscriber::Subscriber;
use thiserror::Error;
use tokio::{
	select, spawn,
	sync::{
		broadcast::{self, error::RecvError},
		mpsc,
	},
	task::AbortHandle,
};

mod control;
mod subscriber;

const MPSC_BUF_SIZE: usize = 32;
const BROADCAST_BUF_SIZE: usize = 32;
const CONTROL_BUF_SIZE: usize = 32;

type BroadcastMessage<M> = Arc<M>;
type MpscMessage<T, M> = (T, PubSubMessage<M>);

pub enum PubSubMessage<M> {
	Ok(Arc<M>),
	Lagged(u64),
	TopicClosed,
}

type Mpsc<T> = (mpsc::Sender<T>, mpsc::Receiver<T>);
type UMpsc<T> = (mpsc::UnboundedSender<T>, mpsc::UnboundedReceiver<T>);

pub struct Publisher<T, M> {
	create_subscriber: Mpsc<CreateSubscriber<T, M>>,
	subscribe: Mpsc<SubscribeTopic<T>>,
	unsubscribe: Mpsc<UnsubscribeTopic<T>>,
	// its fine to have this channel unbounded because
	// each subscriber will only ever call it once at most
	//
	// since this will be sent to from the Drop impl of Subscriber,
	// which is sync, with a bounded channel we would have to spawn a task
	// every time which also allocates so there is no difference between that
	// or having an unbounded channel
	destroy_subscriber: UMpsc<DestroySubscriber>,

	next_subscriber_id: u64,
	subscribers: HashMap<u64, SubscriberData<T, M>>,
	topics: HashMap<T, broadcast::Sender<BroadcastMessage<M>>>,
}

struct SubscriberData<T, M> {
	mpsc_sender: mpsc::Sender<MpscMessage<T, M>>,
	funnel_tasks: HashMap<T, AbortHandle>,
}

pub struct TopicControl<F1, F2> {
	pub on_topic_create: F1,
	pub on_topic_destroy: F2,
}

#[derive(Error, Debug)]
#[error("topic doesnt exist")]
pub struct TopicDoesntExist;

impl<T: Hash + Eq + Clone + Send + Sync + 'static, M: Send + Sync + 'static> Publisher<T, M> {
	pub fn new() -> Self {
		Self {
			create_subscriber: mpsc::channel(CONTROL_BUF_SIZE),
			subscribe: mpsc::channel(CONTROL_BUF_SIZE),
			unsubscribe: mpsc::channel(CONTROL_BUF_SIZE),
			destroy_subscriber: mpsc::unbounded_channel(),

			next_subscriber_id: 0,
			subscribers: HashMap::new(),
			topics: HashMap::new(),
		}
	}
	pub async fn control_step<E, F1, F2>(
		&mut self,
		topic_control: TopicControl<F1, F2>,
	) -> Result<(), E>
	where
		F1: AsyncFnMut(&T) -> Result<(), E>,
		F2: AsyncFnMut(&T) -> Result<(), E>,
	{
		select! {
			// impossible to get None, since there will be always at
			// least one sender in the Publisher struct itself
			Some(create) = self.create_subscriber.1.recv() => {
				// ignore error, which would happen if the oneshot receiver is already dropped
				// the newly created subscriber will be just dropped too, and clean up automatically
				let _ = create.response.send(self.new_subscriber());
			}
			Some(destroy) = self.destroy_subscriber.1.recv() => {
				self.destroy_subscriber(topic_control.on_topic_destroy, destroy.id).await?;
			}
			Some(subscribe) = self.subscribe.1.recv() => {
				self.subscribe(topic_control.on_topic_create, subscribe.subscriber_id, subscribe.topic).await?;
			}
			Some(unsubscribe) = self.unsubscribe.1.recv() => {
				self.unsubscribe(topic_control.on_topic_destroy, unsubscribe.subscriber_id, unsubscribe.topic).await?;
			}
		}

		Ok(())
	}
	pub fn publish(&mut self, topic: &T, message: M) -> Result<(), TopicDoesntExist> {
		let topic_broadcast = match self.topics.get(topic) {
			Some(x) => x,
			None => return Err(TopicDoesntExist),
		};

		// a topic may exist with no subscribers in which case this will error but we ignore
		// because eventually it will get cleaned up

		// the only way for this to fail is if:
		// - All topic broadcast receivers are dropped
		// - Which implies, that the last funnel task with that receiver exited
		// - A funnel task can exit in three ways:
		//    X Aborted: this only happens in unsubscribe/destroy_subscriber methods
		//      which also immediatelly clean up any empty topics
		//    X Topic broadcast is dropped: obviously not in this case because we have it right here
		//    √ Subscriber MPSC receiver dropped, which means that the whole Subscriber was dropped
		// - And when a Subscriber is dropped, it will send a DestroySubscriber control message
		let _ = topic_broadcast.send(Arc::new(message));

		Ok(())
	}
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static, M: Send + Sync + 'static> Publisher<T, M> {
	fn next_subscriber_id(&mut self) -> u64 {
		let id = self.next_subscriber_id;
		self.next_subscriber_id += 1;
		id
	}
	fn new_subscriber(&mut self) -> Subscriber<T, M> {
		Subscriber::new(self)
	}
	async fn destroy_subscriber<F, E>(&mut self, mut on_topic_destroy: F, id: u64) -> Result<(), E>
	where
		F: AsyncFnMut(&T) -> Result<(), E>,
	{
		let subscriber = self
			.subscribers
			.remove(&id)
			.expect("remove non-existing subscriber");

		for (topic, abort_handle) in subscriber.funnel_tasks {
			abort_handle.abort();
			self.cleanup_topic_if_empty(&topic, &mut on_topic_destroy)
				.await?;
		}

		Ok(())
	}
	async fn subscribe<F, E>(&mut self, mut on_topic_create: F, id: u64, topic: T) -> Result<(), E>
	where
		F: AsyncFnMut(&T) -> Result<(), E>,
	{
		let subscriber = self
			.subscribers
			.get_mut(&id)
			.expect("subscribe with non-existing subscriber");

		// do nothing if already subscribed
		if subscriber.funnel_tasks.contains_key(&topic) {
			return Ok(());
		}

		let topic_sender_entry = self.topics.entry(topic.clone());
		let is_new_topic = matches!(topic_sender_entry, Entry::Vacant(_));
		let topic_sender =
			topic_sender_entry.or_insert_with(|| broadcast::Sender::new(BROADCAST_BUF_SIZE));

		let mpsc_sender = subscriber.mpsc_sender.clone();
		let broadcast_receiver = topic_sender.subscribe();

		let handle =
			spawn(funnel_task(topic.clone(), broadcast_receiver, mpsc_sender)).abort_handle();

		subscriber.funnel_tasks.insert(topic.clone(), handle);

		if is_new_topic {
			on_topic_create(&topic).await?;
		}

		Ok(())
	}
	async fn unsubscribe<F, E>(&mut self, on_topic_destroy: F, id: u64, topic: T) -> Result<(), E>
	where
		F: AsyncFnMut(&T) -> Result<(), E>,
	{
		let subscriber = self
			.subscribers
			.get_mut(&id)
			.expect("unsubscribe with non-existing subscriber");

		let abort_handle = subscriber
			.funnel_tasks
			.get(&topic)
			.expect("unsubscribe from topic to which not subscribed to");

		abort_handle.abort();
		self.cleanup_topic_if_empty(&topic, on_topic_destroy)
			.await?;

		Ok(())
	}
	async fn cleanup_topic_if_empty<F, E>(&mut self, topic: &T, f: F) -> Result<(), E>
	where
		F: AsyncFnOnce(&T) -> Result<(), E>,
	{
		let topic_sender = self.topics.get(topic).unwrap();

		if topic_sender.receiver_count() == 0 {
			f(topic).await?;
			self.topics.remove(topic);
		}

		Ok(())
	}
}

// task will end when either the mpsc receiver is dropped or when the broadcast sender is dropped
async fn funnel_task<T: Clone, M>(
	topic: T,
	mut broadcast_receiver: broadcast::Receiver<Arc<M>>,
	mpsc_sender: mpsc::Sender<(T, PubSubMessage<M>)>,
) {
	loop {
		select! {
			_ = mpsc_sender.closed() => break,
			msg = broadcast_receiver.recv() => {
				let mut closed = false;
				let msg = match msg {
					Ok(x) => PubSubMessage::Ok(x),
					Err(RecvError::Lagged(n)) => PubSubMessage::Lagged(n),
					Err(RecvError::Closed) => {
						closed = true;
						PubSubMessage::TopicClosed
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
