use std::convert::Infallible;

use super::subscriber::Subscriber;
use crate::{
	Message, Topic, TopicContext,
	error::{TopicAlreadyAdded, TopicNotSubscribed},
	traits::TopicError,
};
use tokio::sync::oneshot;

pub(crate) enum ControlMessage<T: Topic, M: Message, C: TopicContext, E: TopicError = Infallible> {
	CreateSubscriber {
		response: oneshot::Sender<Subscriber<T, M, C, E>>,
	},
	DestroySubscriber {
		id: u64,
	},
	AddTopic {
		id: u64,
		topic: T,
		response: oneshot::Sender<Result<Result<C, E>, TopicAlreadyAdded>>,
	},
	RemoveTopic {
		id: u64,
		topic: T,
		response: oneshot::Sender<Result<(), TopicNotSubscribed>>,
	},
}
