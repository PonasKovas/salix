use crate::database::{Database, message::Message};
use messages::ChatroomContext;
use sqlx::PgPool;
use std::sync::Arc;
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

impl UpdateSubscriber {
	pub async fn recv_chat_messages(&mut self) -> (Uuid, Arc<Message>) {
		let (chat_id, msg) = self.messages.recv().await.unwrap();

		let msg = match msg {
			tokio_pubsub::PubSubMessage::Ok(x) => x,
			tokio_pubsub::PubSubMessage::Lagged(_n) => todo!(),
		};

		(chat_id, msg)
	}
	pub async fn destroy(self) {
		self.messages.destroy().await;
	}
}
