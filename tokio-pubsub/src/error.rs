use thiserror::Error;

/// When the topic doesn't exist (no subscribers)
#[derive(Error, Debug)]
#[error("topic doesnt exist")]
pub struct TopicDoesntExist;

/// When the associated [`Publisher`][crate::Publisher] instance was dropped
#[derive(Error, Debug)]
#[error("the associated publisher was dropped")]
pub struct PublisherDropped;
