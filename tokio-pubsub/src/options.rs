/// Configuration options
#[derive(Debug, Clone, Copy)]
pub struct Options {
	/// The size of the MPSC channel for each individual subscriber
	pub subscriber_channel_size: usize,
	/// The size of the broadcast channel for each individual topic
	///
	/// If subscribers don't read from this channel fast enough they will lag
	/// and miss messages
	pub topic_broadcast_channel_size: usize,
	/// The size of internal control channel
	///
	/// For example for creating new subscribers, subscribing/unsubscribing to topics etc.
	pub control_channel_size: usize,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			subscriber_channel_size: 64,
			topic_broadcast_channel_size: 32,
			control_channel_size: 32,
		}
	}
}
