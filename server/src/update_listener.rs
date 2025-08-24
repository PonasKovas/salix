use crate::{
	cmd_args::Args,
	config::Config,
	database::{Database, message::Message},
};
use messages::{ChatroomContext, MessagesListener};
use sqlx::PgPool;
use tokio::spawn;
use tokio_pubsub::{Publisher, PublisherHandle, Subscriber};
use tracing::error;
use uuid::Uuid;

mod messages;

#[derive(Clone, Debug)]
pub struct UpdateListener {
	messages: PublisherHandle<Uuid, Message, ChatroomContext>,
}

#[derive(Debug)]
pub struct UpdateSubscriber {
	pub messages: Subscriber<Uuid, Message, ChatroomContext>,
}

impl UpdateListener {
	pub async fn init(_config: &Config, _args: &Args, db: &Database<PgPool>) -> sqlx::Result<Self> {
		let messages_publisher = Publisher::new();
		let messages_handle = messages_publisher.handle();

		let messages_listener = MessagesListener::new(db).await?;
		spawn(async move {
			if let Err(e) = messages_listener.run(messages_publisher).await {
				error!("{e}");
			}
		});

		Ok(Self {
			messages: messages_handle,
		})
	}
	pub async fn subscribe(&self) -> UpdateSubscriber {
		UpdateSubscriber {
			messages: self.messages.subscribe().await.unwrap(),
		}
	}
}
