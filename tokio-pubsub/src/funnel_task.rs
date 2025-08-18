use crate::PubSubMessage;
use std::sync::Arc;
use tokio::{
	select,
	sync::{
		broadcast::{self, error::RecvError},
		mpsc,
	},
};

// task will end when either the mpsc receiver is dropped or when the broadcast sender is dropped
pub(crate) async fn funnel_task<T: Clone, M>(
	topic: T,
	mut broadcast_receiver: broadcast::Receiver<Arc<M>>,
	mpsc_sender: mpsc::Sender<(T, PubSubMessage<M>)>,
) {
	loop {
		select! {
			_ = mpsc_sender.closed() => break,
			msg = broadcast_receiver.recv() => {
				let msg = match msg {
					Ok(x) => PubSubMessage::Ok(x),
					Err(RecvError::Lagged(n)) => PubSubMessage::Lagged(n),
					Err(RecvError::Closed) => break,
				};

				if mpsc_sender.send((topic.clone(), msg)).await.is_err() {
					break;
				}
			}
		}
	}
}
