//! Some trait aliases for better readability

use std::hash::Hash;

/// Types that can be used as a topic id.
///
/// Topic will be cloned a lot, so prefer using cheaply clonable types.
pub trait Topic: Hash + Eq + Clone + Send + Sync + 'static {}
impl<T> Topic for T where T: Hash + Eq + Clone + Send + Sync + 'static {}

/// Types that can be used as messages
///
/// The message will be put into an [`Arc`][std::sync::Arc] before sending
pub trait Message: Send + Sync + 'static {}
impl<T> Message for T where T: Send + Sync + 'static {}

/// Types that can be used as topic context
///
/// Topic context is returned to a subscriber when it subscribes to a new topic
pub trait TopicContext: Send + Sync + 'static {}
impl<T> TopicContext for T where T: Send + Sync + 'static {}

/// Types that can be used as topic subscription error
pub trait TopicError: Send + Sync + 'static {}
impl<T> TopicError for T where T: Send + Sync + 'static {}
