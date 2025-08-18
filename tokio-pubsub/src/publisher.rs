use crate::{
	BroadcastMessage, Message, MpscMessage, Topic,
	control::{CreateSubscriber, DestroySubscriber, SubscribeTopic, UnsubscribeTopic},
	error::TopicDoesntExist,
	funnel_task::funnel_task,
	options::Options,
	publisher_handle::PublisherHandle,
	subscriber::Subscriber,
};
use ahash::{HashMap, HashMapExt};
use std::{collections::hash_map::Entry, marker::PhantomData, sync::Arc, task::Poll};
use tokio::{
	select, spawn,
	sync::{broadcast, mpsc},
	task::AbortHandle,
};

type Mpsc<T> = (mpsc::Sender<T>, mpsc::Receiver<T>);
type UMpsc<T> = (mpsc::UnboundedSender<T>, mpsc::UnboundedReceiver<T>);

/// The entry structure.
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
pub struct Publisher<T, M> {
	pub(crate) options: Options,

	pub(crate) create_subscriber: Mpsc<CreateSubscriber<T, M>>,
	pub(crate) subscribe: Mpsc<SubscribeTopic<T>>,
	pub(crate) unsubscribe: Mpsc<UnsubscribeTopic<T>>,
	// its fine to have this channel unbounded because
	// each subscriber will only ever call it once at most
	//
	// since this will be sent to from the Drop impl of Subscriber,
	// we dont want any backpressure
	pub(crate) destroy_subscriber: UMpsc<DestroySubscriber>,

	next_subscriber_id: u64,
	pub(crate) subscribers: HashMap<u64, SubscriberData<T, M>>,
	topics: HashMap<T, broadcast::Sender<BroadcastMessage<M>>>,
}

pub(crate) struct SubscriberData<T, M> {
	pub mpsc_sender: mpsc::Sender<MpscMessage<T, M>>,
	pub funnel_tasks: HashMap<T, AbortHandle>,
}

impl<T: Topic, M: Message> Publisher<T, M> {
	/// Creates a new [`Publisher`] with the default options
	pub fn new() -> Self {
		Self::with_options(Default::default())
	}
	/// Creates a new [`Publisher`] with the given options
	pub fn with_options(options: Options) -> Self {
		Self {
			options,

			create_subscriber: mpsc::channel(options.control_channels_size),
			subscribe: mpsc::channel(options.control_channels_size),
			unsubscribe: mpsc::channel(options.control_channels_size),
			destroy_subscriber: mpsc::unbounded_channel(),

			next_subscriber_id: 0,
			subscribers: HashMap::new(),
			topics: HashMap::new(),
		}
	}
	/// Gets a new [`PublisherHandle`] to the current [`Publisher`].
	///
	/// This handle can be used to manage subscribers,
	/// cloning it just returns a handle to the same [`Publisher`].
	pub fn handle(&self) -> PublisherHandle<T, M> {
		PublisherHandle::new(self)
	}
	/// Handles the next control step
	///
	/// This must be called in a loop to keep the [`Publisher`] functioning,
	/// it handles creating/destroying subscribers, handling new subscriptions
	/// to topics.
	///
	/// You can add your own topic setup/removal logic using [`TopicControl`]
	pub async fn control_step<E, F1, F2>(
		&mut self,
		topic_control: TopicControl<T, E, F1, F2>,
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
	/// Publishes a new message to a certain topic.
	///
	/// This will error if the topic doesn't exist (there are no subscribers to it).
	/// Generally you should keep track of what topics are subscribed to using [`TopicControl`] in
	/// [`Publisher::control_step`].
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
}

impl<T: Topic, M: Message> Publisher<T, M> {
	pub(crate) fn next_subscriber_id(&mut self) -> u64 {
		let id = self.next_subscriber_id;
		self.next_subscriber_id += 1;
		id
	}
	pub(crate) fn new_subscriber(&mut self) -> Subscriber<T, M> {
		Subscriber::new(self)
	}
	pub(crate) async fn destroy_subscriber<F, E>(
		&mut self,
		mut on_topic_destroy: F,
		id: u64,
	) -> Result<(), E>
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
	pub(crate) async fn subscribe<F, E>(
		&mut self,
		mut on_topic_create: F,
		id: u64,
		topic: T,
	) -> Result<(), E>
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
			.expect("subscribe with non-existing subscriber");

		// do nothing if already subscribed
		if subscriber.funnel_tasks.contains_key(&topic) {
			return Ok(());
		}

		let topic_sender_entry = self.topics.entry(topic.clone());
		let is_new_topic = matches!(topic_sender_entry, Entry::Vacant(_));
		let topic_sender = topic_sender_entry
			.or_insert_with(|| broadcast::Sender::new(self.options.topic_broadcast_channel_size));

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
	pub(crate) async fn unsubscribe<F, E>(
		&mut self,
		mut on_topic_destroy: F,
		id: u64,
		topic: T,
	) -> Result<(), E>
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
			// if not subscribed to the topic, just do nothing
			None => return Ok(()),
		};

		abort_handle.abort();
		self.cleanup_topic_if_empty(&topic, &mut on_topic_destroy)
			.await?;

		Ok(())
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

struct Nothing<E>(PhantomData<*mut E>);
impl<E> Future for Nothing<E> {
	type Output = Result<(), E>;

	fn poll(
		self: std::pin::Pin<&mut Self>,
		_cx: &mut std::task::Context<'_>,
	) -> Poll<Self::Output> {
		Poll::Ready(Ok(()))
	}
}
fn do_nothing<T, E>(_: &T) -> Nothing<E> {
	Nothing(PhantomData)
}

/// Closures to be run on the events of new topics being subscribed to or
/// when the last subscriber of a topic unsubscribes
#[allow(private_interfaces)]
pub struct TopicControl<T, E, F1 = fn(&T) -> Nothing<E>, F2 = fn(&T) -> Nothing<E>>
where
	T: Topic,
	F1: AsyncFnMut(&T) -> Result<(), E>,
	F2: AsyncFnMut(&T) -> Result<(), E>,
{
	/// Runs when a new topic is first subscribed to
	pub on_topic_create: F1,
	/// Runs when the last subscriber unsubscribes from a topic
	pub on_topic_destroy: F2,

	pub phantom: PhantomData<*mut (T, E)>,
}

impl<T: Topic, E> Default for TopicControl<T, E> {
	fn default() -> Self {
		Self {
			on_topic_create: do_nothing,
			on_topic_destroy: do_nothing,
			phantom: PhantomData,
		}
	}
}
