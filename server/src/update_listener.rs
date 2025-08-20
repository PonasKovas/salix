use crate::{
	cmd_args::Args,
	config::Config,
	db::{Database, message::Message},
};
use tokio::{
	spawn,
	sync::{mpsc, oneshot},
};
use tokio_pubsub::{Publisher, PublisherHandle};
use tracing::error;
use uuid::Uuid;

mod messages;

#[derive(Clone, Debug)]
pub struct UpdateListener {
	messages: PublisherHandle<Uuid, Message, i64>,
}

impl UpdateListener {
	pub async fn init(_config: &Config, _args: &Args, db: &Database) -> sqlx::Result<Self> {
		let publisher = Publisher::new();
		let handle = publisher.handle();

		let worker = ListenerWorker::new(db, publisher).await?;
		spawn(async move {
			if let Err(e) = worker.run().await {
				error!("{e}");
			}
		});

		Ok(Self { messages: handle })
	}
	// /// returns the last message that was in the chat sequence id
	// pub async fn subscribe(&self) -> (i64, MessagesSubscriber) {
	// 	let (oneshot_sender, oneshot_receiver) = oneshot::channel();
	// 	self.control
	// 		.send(ControlMessage::SubscribeToChat {
	// 			respond: oneshot_sender,
	// 		})
	// 		.await
	// 		.unwrap();

	// 	let response = oneshot_receiver.await.unwrap();

	// 	(
	// 		response.last_message_seq_id,
	// 		MessagesSubscriber {
	// 			receiver: response.updates,
	// 		},
	// 	)
	// }
}
