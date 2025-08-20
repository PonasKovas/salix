use crate::{
	BroadcastMessage, Message, MpscMessage, Topic, TopicContext,
	control::ControlMessage,
	error::{TopicAlreadyAdded, TopicDoesntExist, TopicNotSubscribed},
	funnel_task::funnel_task,
	options::Options,
	publisher_handle::PublisherHandle,
	subscriber::Subscriber,
};
use ahash::{HashMap, HashMapExt};
use std::sync::Arc;
use tokio::{
	spawn,
	sync::{broadcast, mpsc},
	task::AbortHandle,
};

/// The main structure.
///
/// A publisher manages subscribers and topics, publishes messages.
///
/// # `T` parameter
///
/// `T` is the topic type, it will be cloned a lot so prefer using cheaply-cloneable
/// types like integer IDs, instead of something like [`String`].
///
/// # `M` parameter
///
/// `M` is the message type, used for messages in all topics. It will never be cloned,
/// and instead be wrapped in an [`Arc`] before sending.
///
/// # `C` parameter
///
/// `C` is the topic context type, returned to a subscriber when it subscribes to a topic
/// and may indicate failure by using something like `Result<T, Err>`.
pub struct Publisher<T: Topic, M: Message, C: TopicContext> {
	options: Options,

	control_sender: mpsc::Sender<ControlMessage<T, M, C>>,
	control_receiver: mpsc::Receiver<ControlMessage<T, M, C>>,

	next_subscriber_id: u64,
	subscribers: HashMap<u64, SubscriberData<T, M>>,
	topics: HashMap<T, broadcast::Sender<BroadcastMessage<M>>>,
}

struct SubscriberData<T, M> {
	mpsc_sender: mpsc::Sender<MpscMessage<T, M>>,
	funnel_tasks: HashMap<T, AbortHandle>,
}

impl<T: Topic, M: Message, C: TopicContext> Publisher<T, M, C> {
	/// Creates a new [`Publisher`] with the default options
	pub fn new() -> Self {
		Self::with_options(Default::default())
	}
	/// Creates a new [`Publisher`] with the given options
	pub fn with_options(options: Options) -> Self {
		let (control_sender, control_receiver) = mpsc::channel(options.control_channel_size);

		Self {
			options,

			control_sender,
			control_receiver,

			next_subscriber_id: 0,
			subscribers: HashMap::new(),
			topics: HashMap::new(),
		}
	}
	/// Gets a new [`PublisherHandle`] to the current [`Publisher`].
	///
	/// This handle can be used to manage subscribers,
	/// cloning it just returns a handle to the same [`Publisher`].
	pub fn handle(&self) -> PublisherHandle<T, M, C> {
		PublisherHandle::new(self.control_sender.clone())
	}
	/// Drives the publisher one step, handling operations like creating/removing subscribers, etc
	///
	/// **This must be called in a loop** to keep the [`Publisher`] functioning,
	/// it handles creating/destroying subscribers, handling new subscriptions
	/// to topics.
	///
	/// You can add your own topic setup/removal logic and send topic context using [`TopicControl`]
	pub async fn drive<E, F1, F2>(&mut self, topic_control: TopicControl<F1, F2>) -> Result<(), E>
	where
		F1: AsyncFnMut(&T) -> Result<C, E>,
		F2: AsyncFnMut(&T) -> Result<(), E>,
	{
		// impossible to get None, since there will be always at
		// least one sender in the Publisher struct itself
		let control_msg = self.control_receiver.recv().await.unwrap();
		match control_msg {
			ControlMessage::CreateSubscriber { response } => {
				if let Err(subscriber) = response.send(self.new_subscriber()) {
					subscriber.destroy().await;
				}
			}
			ControlMessage::DestroySubscriber { id } => {
				self.destroy_subscriber(topic_control.on_topic_unsubscribe, id)
					.await?;
			}
			ControlMessage::AddTopic {
				id,
				topic,
				response,
			} => {
				let r = self
					.add_topic(topic_control.on_topic_subscribe, id, topic)
					.await?;

				// if the oneshot receiver already dropped, the whole subscriber must
				// be dropped, and it will be cleaned up automatically, so ignore errors
				let _ = response.send(r);
			}
			ControlMessage::RemoveTopic {
				id,
				topic,
				response,
			} => {
				let r = self
					.remove_topic(topic_control.on_topic_unsubscribe, id, topic)
					.await?;

				// if the oneshot receiver already dropped, the whole subscriber must
				// be dropped, and it will be cleaned up automatically, so ignore errors
				let _ = response.send(r);
			}
		}

		Ok(())
	}
	/// Publishes a new message to a certain topic.
	///
	/// This will error if the topic doesn't exist (there are no subscribers to it).
	/// Generally you should keep track of what topics are subscribed to using [`TopicControl`] in
	/// [`Publisher::drive`].
	pub fn publish(&mut self, topic: &T, message: M) -> Result<(), TopicDoesntExist> {
		let topic_broadcast = match self.topics.get(topic) {
			Some(x) => x,
			None => return Err(TopicDoesntExist),
		};

		// the only way for this to fail is if:
		// - All topic broadcast receivers are dropped
		// - Which implies, that the last funnel task with that receiver exited
		// - A funnel task can exit in three ways:
		//    X Aborted: this only happens in unsubscribe/destroy_subscriber methods
		//      which also immediatelly clean up any empty topics
		//    X Topic broadcast sender is dropped: obviously not in this case because we have it right here
		//    √ Subscriber MPSC receiver dropped, which means that the whole Subscriber was dropped
		// - And when a Subscriber is dropped, it will send a DestroySubscriber control message
		//
		// so if this fails that means the last Subscriber was dropped but we just havent removed it yet
		// but we will soon so its fine
		let _ = topic_broadcast.send(Arc::new(message));

		Ok(())
	}
	/// Creates a new [`Subscriber`] to this publisher.
	///
	/// Note that calling control methods on the [`Subscriber`] requires
	/// driving this publisher.
	pub fn new_subscriber(&mut self) -> Subscriber<T, M, C> {
		let (sender, receiver) = mpsc::channel(self.options.subscriber_channel_size);

		let id = self.next_subscriber_id();
		self.subscribers.insert(
			id,
			SubscriberData {
				mpsc_sender: sender,
				funnel_tasks: HashMap::new(),
			},
		);

		Subscriber::new(id, self.control_sender.clone(), receiver)
	}
}

impl<T: Topic, M: Message, C: TopicContext> Publisher<T, M, C> {
	fn next_subscriber_id(&mut self) -> u64 {
		let id = self.next_subscriber_id;
		self.next_subscriber_id += 1;
		id
	}
	async fn destroy_subscriber<F, E>(&mut self, mut on_topic_destroy: F, id: u64) -> Result<(), E>
	where
		F: AsyncFnMut(&T) -> Result<(), E>,
	{
		// This should never fail, because the only way to call to destroy a subscriber
		// is by dropping the Subscriber instance, and the only way to obtain a Subscriber
		// instance involves adding SubscriberData to this map
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
	async fn add_topic<F, E>(
		&mut self,
		mut on_topic_create: F,
		id: u64,
		topic: T,
	) -> Result<Result<C, TopicAlreadyAdded>, E>
	where
		F: AsyncFnMut(&T) -> Result<C, E>,
	{
		// This should never fail, because the only way to call this function is through a living
		// Subscriber instance, and the only way to obtain a Subscriber
		// instance involves adding SubscriberData to this map
		// which will not be removed until the Subscriber is dropped
		let subscriber = self
			.subscribers
			.get_mut(&id)
			.expect("subscribe with non-existing subscriber");

		// if already subscribed
		if subscriber.funnel_tasks.contains_key(&topic) {
			return Ok(Err(TopicAlreadyAdded));
		}

		let context = on_topic_create(&topic).await?;

		// dont actually subscribe to the topic if failure
		if context.is_failure() {
			return Ok(Ok(context));
		}

		let topic_sender = self
			.topics
			.entry(topic.clone())
			.or_insert_with(|| broadcast::Sender::new(self.options.topic_broadcast_channel_size));

		let mpsc_sender = subscriber.mpsc_sender.clone();
		let broadcast_receiver = topic_sender.subscribe();

		let handle =
			spawn(funnel_task(topic.clone(), broadcast_receiver, mpsc_sender)).abort_handle();

		subscriber.funnel_tasks.insert(topic.clone(), handle);

		Ok(Ok(context))
	}
	async fn remove_topic<F, E>(
		&mut self,
		mut on_topic_destroy: F,
		id: u64,
		topic: T,
	) -> Result<Result<(), TopicNotSubscribed>, E>
	where
		F: AsyncFnMut(&T) -> Result<(), E>,
	{
		// This should never fail, because the only way to call this function is through a living
		// Subscriber instance, and the only way to obtain a Subscriber
		// instance involves adding SubscriberData to this map
		// which will not be removed until the Subscriber is dropped
		let subscriber = self
			.subscribers
			.get_mut(&id)
			.expect("unsubscribe with non-existing subscriber");

		let abort_handle = match subscriber.funnel_tasks.get(&topic) {
			Some(x) => x,
			// if not subscribed to the topic
			None => return Ok(Err(TopicNotSubscribed)),
		};

		abort_handle.abort();
		self.cleanup_topic_if_empty(&topic, &mut on_topic_destroy)
			.await?;

		Ok(Ok(()))
	}
	/// If a given topic has no more subscribers, removes it completely
	async fn cleanup_topic_if_empty<F, E>(&mut self, topic: &T, f: &mut F) -> Result<(), E>
	where
		F: AsyncFnMut(&T) -> Result<(), E>,
	{
		let topic_sender = match self.topics.get(topic) {
			Some(x) => x,
			// if the topic doesnt exist there is nothing to clean anyway, just ignore
			None => return Ok(()),
		};

		if topic_sender.receiver_count() == 0 {
			f(topic).await?;
			self.topics.remove(topic);
		}

		Ok(())
	}
}

/// Closures to be run on the events of topics being subscribed to
/// or unsubscribed from
pub struct TopicControl<F1, F2> {
	/// Runs when a topic is subscribed to
	pub on_topic_subscribe: F1,
	/// Runs when a topic is unsubscribed from
	pub on_topic_unsubscribe: F2,
}
