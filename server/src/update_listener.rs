use crate::database::{Database, message::Message};
use futures::StreamExt;
use messages::ChatroomContext;
use sqlx::PgPool;
use std::{
	collections::{BTreeMap, VecDeque},
	sync::Arc,
};
use tokio_pubsub::{PublisherHandle, Subscriber};
use uuid::Uuid;

mod messages;

#[derive(Clone, Debug)]
pub struct UpdateListener {
	database: Database<PgPool>,
	messages: PublisherHandle<Uuid, Message, ChatroomContext>,
}

#[derive(Debug)]
pub struct UpdateSubscriber {
	database: Database<PgPool>,

	messages: Subscriber<Uuid, Message, ChatroomContext>,
	messages_last_seq_ids: BTreeMap<Uuid, i64>,
	messages_buffer: VecDeque<(Uuid, Arc<Message>)>,
}

impl UpdateListener {
	pub async fn init(db: &Database<PgPool>) -> sqlx::Result<Self> {
		Ok(Self {
			database: db.clone(),
			messages: messages::start(db).await?,
		})
	}
	pub async fn subscribe(&self) -> UpdateSubscriber {
		UpdateSubscriber {
			database: self.database.clone(),

			messages: self.messages.subscribe().await.unwrap(),
			messages_last_seq_ids: BTreeMap::new(),
			messages_buffer: VecDeque::new(),
		}
	}
}

impl UpdateSubscriber {
	pub async fn recv_chat_messages(&mut self) -> sqlx::Result<(Uuid, Arc<Message>)> {
		if let Some(msg) = self.messages_buffer.pop_front() {
			return Ok(msg);
		}

		let (chat_id, msg) = self.messages.recv().await.unwrap();

		let msg = match msg {
			tokio_pubsub::PubSubMessage::Ok(x) => x,
			tokio_pubsub::PubSubMessage::Lagged(n) => {
				assert!(n != 0);

				let last_seq_id = self.messages_last_seq_ids.get_mut(&chat_id).unwrap();

				let fetch_since = *last_seq_id;
				let fetch_to = fetch_since + n as i64;

				{
					let mut stream = self.database.messages_by_seq_id(fetch_since..fetch_to);

					while let Some(msg) = stream.next().await {
						let msg = msg?;

						*last_seq_id = msg.sequence_id;
						self.messages_buffer.push_back((chat_id, Arc::new(msg)));
					}
				}

				return Ok(self.messages_buffer.pop_front().unwrap());
			}
		};

		*self.messages_last_seq_ids.get_mut(&chat_id).unwrap() = msg.sequence_id;

		Ok((chat_id, msg))
	}
	pub async fn subscribe_chat(
		&mut self,
		chat_id: Uuid,
	) -> Result<(), tokio_pubsub::error::TopicAlreadyAdded> {
		match self.messages.add_topic(chat_id).await {
			Ok(ctx) => {
				self.messages_last_seq_ids
					.insert(chat_id, ctx.last_message_seq_id);

				Ok(())
			}
			Err(tokio_pubsub::error::AddTopicError::AlreadyAdded(e)) => Err(e),
			Err(other) => panic!("{other}"),
		}
	}
	pub async fn unsubscribe_chat(
		&mut self,
		chat_id: Uuid,
	) -> Result<(), tokio_pubsub::error::TopicNotSubscribed> {
		match self.messages.remove_topic(chat_id).await {
			Ok(()) => {
				self.messages_last_seq_ids.remove(&chat_id);

				Ok(())
			}
			Err(tokio_pubsub::error::RemoveTopicError::NotSubscribed(e)) => Err(e),
			Err(other) => panic!("{other}"),
		}
	}
	pub async fn destroy(self) {
		self.messages.destroy().await;
	}
}
