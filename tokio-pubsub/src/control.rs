use super::subscriber::Subscriber;
use crate::{
	Message, Topic, TopicContext,
	error::{TopicAlreadyAdded, TopicNotSubscribed},
};
use tokio::sync::oneshot;

pub(crate) enum ControlMessage<T: Topic, M: Message, C: TopicContext> {
	CreateSubscriber {
		response: oneshot::Sender<Subscriber<T, M, C>>,
	},
	DestroySubscriber {
		id: u64,
	},
	AddTopic {
		id: u64,
		topic: T,
		response: oneshot::Sender<Result<C, TopicAlreadyAdded>>,
	},
	RemoveTopic {
		id: u64,
		topic: T,
		response: oneshot::Sender<Result<(), TopicNotSubscribed>>,
	},
}
