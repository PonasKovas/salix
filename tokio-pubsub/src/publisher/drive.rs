use super::Publisher;
use crate::{Message, Topic, TopicContext, TopicError, control::ControlMessage};
use std::convert::Infallible;

#[allow(async_fn_in_trait)]
pub trait EventReactor<T: Topic, C: TopicContext, E: TopicError> {
	type Error;

	async fn on_subscribe(&mut self, topic: &T) -> Result<Result<C, E>, Self::Error>;
	async fn on_unsubscribe(&mut self, #[allow(unused)] topic: &T) -> Result<(), Self::Error> {
		Ok(())
	}
}

impl<T: Topic, E: TopicError> EventReactor<T, (), E> for () {
	type Error = Infallible;

	async fn on_subscribe(&mut self, _topic: &T) -> Result<Result<(), E>, Self::Error> {
		Ok(Ok(()))
	}
}

#[must_use]
pub struct PublisherDriver<'a, T: Topic, M: Message, C: TopicContext, E: TopicError> {
	publisher: &'a mut Publisher<T, M, C, E>,
	message: ControlMessage<T, M, C, E>,
}

impl<'a, T: Topic, M: Message, C: TopicContext, E: TopicError> PublisherDriver<'a, T, M, C, E> {
	pub(super) fn new(
		publisher: &'a mut Publisher<T, M, C, E>,
		control_message: ControlMessage<T, M, C, E>,
	) -> PublisherDriver<'a, T, M, C, E> {
		PublisherDriver {
			publisher,
			message: control_message,
		}
	}

	pub async fn finish<R>(self, reactor: R) -> Result<(), R::Error>
	where
		R: EventReactor<T, C, E>,
	{
		match self.message {
			ControlMessage::CreateSubscriber(create_subscriber) => {
				let subscriber = self.publisher.new_subscriber();

				let r = create_subscriber.response.send(subscriber);

				if let Err(subscriber) = r {
					subscriber.destroy().await;
				}

				Ok(())
			}
			ControlMessage::DestroySubscriber(destroy_subscriber) => {
				self.publisher
					.destroy_subscriber(destroy_subscriber.id, reactor)
					.await
			}
			ControlMessage::AddTopic(add_topic) => {
				let r = self
					.publisher
					.add_topic(add_topic.id, add_topic.topic, reactor)
					.await?;

				// if the oneshot receiver already dropped, the whole subscriber must
				// be dropped, and it will be cleaned up automatically, so ignore errors
				let _ = add_topic.response.send(r);

				Ok(())
			}
			ControlMessage::RemoveTopic(remove_topic) => {
				let r = self
					.publisher
					.remove_topic(remove_topic.id, remove_topic.topic, reactor)
					.await?;

				// if the oneshot receiver already dropped, the whole subscriber must
				// be dropped, and it will be cleaned up automatically, so ignore errors
				let _ = remove_topic.response.send(r);

				Ok(())
			}
		}
	}
}
