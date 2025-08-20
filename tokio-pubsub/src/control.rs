use super::subscriber::Subscriber;
use crate::{
	Message, Topic, TopicContext,
	error::{TopicAlreadyAdded, TopicNotSubscribed},
	traits::TopicError,
};
use tokio::sync::oneshot;

pub(crate) enum ControlMessage<T: Topic, M: Message, C: TopicContext, E: TopicError> {
	CreateSubscriber(CreateSubscriber<T, M, C, E>),
	DestroySubscriber(DestroySubscriber),
	AddTopic(AddTopic<T, C, E>),
	RemoveTopic(RemoveTopic<T>),
}

pub(crate) struct CreateSubscriber<T: Topic, M: Message, C: TopicContext, E: TopicError> {
	pub(crate) response: oneshot::Sender<Subscriber<T, M, C, E>>,
}
pub(crate) struct DestroySubscriber {
	pub(crate) id: u64,
}
pub(crate) struct AddTopic<T: Topic, C: TopicContext, E: TopicError> {
	pub(crate) id: u64,
	pub(crate) topic: T,
	pub(crate) response: oneshot::Sender<Result<Result<C, E>, TopicAlreadyAdded>>,
}
pub(crate) struct RemoveTopic<T: Topic> {
	pub(crate) id: u64,
	pub(crate) topic: T,
	pub(crate) response: oneshot::Sender<Result<(), TopicNotSubscribed>>,
}
