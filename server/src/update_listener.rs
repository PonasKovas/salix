use crate::{
	cmd_args::Args,
	config::Config,
	db::{Database, message::Message},
};
use std::sync::Arc;
use tokio::{
	spawn,
	sync::{broadcast, mpsc, oneshot},
};
use tracing::error;
use worker::{ControlMessage, ListenerWorker};

mod pubsub;
mod worker;

#[derive(Clone, Debug)]
pub struct UpdateListener {
	control: mpsc::Sender<ControlMessage>,
}

pub struct MessagesSubscriber {
	receiver: broadcast::Receiver<Arc<Message>>,
}

impl UpdateListener {
	pub async fn init(_config: &Config, _args: &Args, db: &Database) -> sqlx::Result<Self> {
		let (control_sender, control_receiver) = mpsc::channel(16);

		let worker = ListenerWorker::new(db).await?;
		spawn(async move {
			if let Err(e) = worker.start(control_receiver).await {
				error!("{e}");
			}
		});

		Ok(Self {
			control: control_sender,
		})
	}
	/// returns the last message that was in the chat sequence id
	pub async fn subscribe_to_chat(&self) -> (i64, MessagesSubscriber) {
		let (oneshot_sender, oneshot_receiver) = oneshot::channel();
		self.control
			.send(ControlMessage::SubscribeToChat {
				respond: oneshot_sender,
			})
			.await
			.unwrap();

		let response = oneshot_receiver.await.unwrap();

		(
			response.last_message_seq_id,
			MessagesSubscriber {
				receiver: response.updates,
			},
		)
	}
}
