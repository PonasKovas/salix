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
pub trait TopicContext: Send + Sync + 'static {
	/// Returns `true` if the topic subscription failed and should not be added to the subscriber.
	///
	/// For an infallible topic, just return `false`.
	fn is_failure(&self) -> bool;
}
impl<T, E> TopicContext for Result<T, E>
where
	T: Send + Sync + 'static,
	E: Send + Sync + 'static,
{
	fn is_failure(&self) -> bool {
		self.is_err()
	}
}
impl<T> TopicContext for Option<T>
where
	T: Send + Sync + 'static,
{
	fn is_failure(&self) -> bool {
		self.is_none()
	}
}
impl TopicContext for () {
	fn is_failure(&self) -> bool {
		false
	}
}
