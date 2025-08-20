use super::Publisher;
use crate::{
	Message, Topic, TopicContext, TopicError,
	control::{AddTopic, ControlMessage, CreateSubscriber, DestroySubscriber, RemoveTopic},
};

#[must_use]
pub struct PublisherDriver<
	'a,
	T: Topic,
	M: Message,
	C: TopicContext,
	E: TopicError,
	ERR,
	// whether on_topic_subscribe was used, to prevent from using it twice
	const ON_SUB: bool = false,
	// same for on_topic_unsubscribe
	const ON_UNSUB: bool = false,
> {
	publisher: &'a mut Publisher<T, M, C, E>,
	create_subscriber: Option<CreateSubscriber<T, M, C, E>>,
	destroy_subscriber: Option<DestroySubscriber>,
	add_topic: Option<AddTopic<T, C, E>>,
	remove_topic: Option<RemoveTopic<T>>,
	result: Result<(), ERR>,
}

impl<'a, T: Topic, M: Message, C: TopicContext, E: TopicError, ERR>
	PublisherDriver<'a, T, M, C, E, ERR, false, false>
{
	pub(super) fn new(
		publisher: &'a mut Publisher<T, M, C, E>,
		control_message: ControlMessage<T, M, C, E>,
	) -> PublisherDriver<'a, T, M, C, E, ERR, false, false> {
		let mut s = PublisherDriver {
			publisher,
			create_subscriber: None,
			destroy_subscriber: None,
			add_topic: None,
			remove_topic: None,
			result: Ok(()),
		};

		match control_message {
			ControlMessage::CreateSubscriber(x) => {
				s.create_subscriber = Some(x);
			}
			ControlMessage::DestroySubscriber(x) => {
				s.destroy_subscriber = Some(x);
			}
			ControlMessage::AddTopic(x) => {
				s.add_topic = Some(x);
			}
			ControlMessage::RemoveTopic(x) => {
				s.remove_topic = Some(x);
			}
		}

		s
	}
}

impl<
	'a,
	T: Topic,
	M: Message,
	C: TopicContext,
	E: TopicError,
	ERR,
	const ON_SUB: bool,
	const ON_UNSUB: bool,
> PublisherDriver<'a, T, M, C, E, ERR, ON_SUB, ON_UNSUB>
{
	async fn handle_create_subscriber(&mut self) {
		if let Some(create_subscriber) = self.create_subscriber.take() {
			let r = create_subscriber
				.response
				.send(self.publisher.new_subscriber());
			if let Err(subscriber) = r {
				subscriber.destroy().await;
			}
		}
	}
	async fn handle_destroy_subscriber<F>(&mut self, on_topic_unsubscribe: F)
	where
		F: AsyncFnMut(&T) -> Result<(), ERR>,
	{
		if let Some(destroy_subscriber) = self.destroy_subscriber.take() {
			self.result = self
				.publisher
				.destroy_subscriber(destroy_subscriber.id, on_topic_unsubscribe)
				.await;
		}
	}
	async fn handle_add_topic<F>(&mut self, on_topic_subscribe: F)
	where
		F: AsyncFnMut(&T) -> Result<Result<C, E>, ERR>,
	{
		if let Some(add_topic) = self.add_topic.take() {
			let r = match self
				.publisher
				.add_topic(add_topic.id, add_topic.topic, on_topic_subscribe)
				.await
			{
				Ok(x) => x,
				Err(e) => {
					self.result = Err(e);
					return;
				}
			};

			// if the oneshot receiver already dropped, the whole subscriber must
			// be dropped, and it will be cleaned up automatically, so ignore errors
			let _ = add_topic.response.send(r);
		}
	}
	async fn handle_remove_topic<F>(&mut self, on_topic_unsubscribe: F)
	where
		F: AsyncFnMut(&T) -> Result<(), ERR>,
	{
		if let Some(remove_topic) = self.remove_topic.take() {
			let r = match self
				.publisher
				.remove_topic(remove_topic.id, remove_topic.topic, on_topic_unsubscribe)
				.await
			{
				Ok(x) => x,
				Err(e) => {
					self.result = Err(e);
					return;
				}
			};

			// if the oneshot receiver already dropped, the whole subscriber must
			// be dropped, and it will be cleaned up automatically, so ignore errors
			let _ = remove_topic.response.send(r);
		}
	}
	fn transition_state<const ON_SUB2: bool, const ON_UNSUB2: bool>(
		self,
	) -> PublisherDriver<'a, T, M, C, E, ERR, ON_SUB2, ON_UNSUB2> {
		PublisherDriver {
			publisher: self.publisher,
			create_subscriber: self.create_subscriber,
			destroy_subscriber: self.destroy_subscriber,
			add_topic: self.add_topic,
			remove_topic: self.remove_topic,
			result: self.result,
		}
	}
	pub async fn on_topic_subscribe<F>(
		mut self,
		f: F,
	) -> PublisherDriver<'a, T, M, C, E, ERR, true, ON_UNSUB>
	where
		F: AsyncFnMut(&T) -> Result<Result<C, E>, ERR>,
		Self: IsInState<false, ON_UNSUB>,
	{
		self.handle_add_topic(f).await;

		self.transition_state()
	}
	pub async fn on_topic_unsubscribe<F>(
		mut self,
		mut f: F,
	) -> PublisherDriver<'a, T, M, C, E, ERR, ON_SUB, true>
	where
		F: AsyncFnMut(&T) -> Result<(), ERR>,
		Self: IsInState<ON_SUB, false>,
	{
		self.handle_destroy_subscriber(&mut f).await;
		self.handle_remove_topic(&mut f).await;

		self.transition_state()
	}
	pub async fn finish(mut self) -> Result<(), ERR>
	where
		Self: IsInState<true, ON_UNSUB>,
	{
		self.handle_create_subscriber().await;
		self.handle_destroy_subscriber(async move |_: &T| Ok(()))
			.await;
		self.handle_remove_topic(async move |_: &T| Ok(())).await;

		return self.result;
	}
}

pub trait IsInState<const ON_SUB: bool, const ON_UNSUB: bool> {}
impl<
	'a,
	T: Topic,
	M: Message,
	C: TopicContext,
	E: TopicError,
	ERR,
	const ON_SUB: bool,
	const ON_UNSUB: bool,
> IsInState<ON_SUB, ON_UNSUB> for PublisherDriver<'a, T, M, C, E, ERR, ON_SUB, ON_UNSUB>
{
}
