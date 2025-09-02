use crate::{
	cmd_args::Args,
	config::Config,
	database::{Database, message::Message},
};
use messages::ChatroomContext;
use sqlx::PgPool;
use tokio_pubsub::{PublisherHandle, Subscriber};
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
	pub async fn init(db: &Database<PgPool>) -> sqlx::Result<Self> {
		Ok(Self {
			messages: messages::start(db).await?,
		})
	}
	pub async fn subscribe(&self) -> UpdateSubscriber {
		UpdateSubscriber {
			messages: self.messages.subscribe().await.unwrap(),
		}
	}
}
