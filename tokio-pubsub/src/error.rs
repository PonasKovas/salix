use thiserror::Error;

/// When the topic doesn't exist (no subscribers)
#[derive(Error, Debug)]
#[error("topic doesnt exist")]
pub struct TopicDoesntExist;

/// When the associated [`Publisher`][crate::Publisher] instance was dropped
#[derive(Error, Debug)]
#[error("the associated publisher was dropped")]
pub struct PublisherDropped;

/// When the topic is already added
#[derive(Error, Debug)]
#[error("topic already added")]
pub struct TopicAlreadyAdded;

/// When trying to remove a topic which is not added
#[derive(Error, Debug)]
#[error("topic is not subscribed")]
pub struct TopicNotSubscribed;

/// Errors that [`Subscriber::add_topic`][crate::Subscriber::add_topic] can return
#[derive(Error, Debug)]
pub enum AddTopicError<E> {
	#[error(transparent)]
	AlreadyAdded(#[from] TopicAlreadyAdded),
	#[error(transparent)]
	PublisherDropped(#[from] PublisherDropped),
	#[error(transparent)]
	TopicError(E),
}

/// Errors that [`Subscriber::remove_topic`][crate::Subscriber::remove_topic] can return
#[derive(Error, Debug)]
pub enum RemoveTopicError {
	#[error(transparent)]
	NotSubscribed(#[from] TopicNotSubscribed),
	#[error(transparent)]
	PublisherDropped(#[from] PublisherDropped),
}
