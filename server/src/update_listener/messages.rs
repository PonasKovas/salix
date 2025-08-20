use crate::db::{Database, message::Message};
use ahash::{HashMap, HashMapExt};
use anyhow::bail;
use chrono::{DateTime, Local};
use serde::Deserialize;
use sqlx::postgres::PgListener;
use std::{collections::hash_map::Entry, convert::Infallible, sync::Arc};
use tokio::{
	select,
	sync::{broadcast, mpsc, oneshot},
};
use tokio_pubsub::{Publisher, TopicControl};
use tracing::error;
use uuid::Uuid;

pub struct MessagesListener {
	db: PgListener,
	publisher: Publisher<Uuid, Message, i64>,
	chatrooms: HashMap<Uuid, ChatroomState>,
}

struct ChatroomState {
	listeners_n: u32,
	last_message_seq_id: i64,
}

struct ChatroomTopicControl<'a> {
	db: &'a mut PgListener,
	chatrooms: &'a mut HashMap<Uuid, ChatroomState>,
}
impl<'a> TopicControl<Uuid, i64, Infallible> for ChatroomTopicControl<'a> {
	type Error = sqlx::Error;

	async fn on_topic_subscribe(
		&mut self,
		topic: &Uuid,
	) -> Result<Result<i64, Infallible>, Self::Error> {
		let chatroom_data = self
			.chatrooms
			.entry(topic.clone())
			.or_insert_with(|| ChatroomState {
				listeners_n: 0,
				last_message_seq_id: -1,
			});
		chatroom_data.listeners_n += 1;

		Ok(Ok(chatroom_data.last_message_seq_id))
	}
	async fn on_topic_unsubscribe(&mut self, topic: &Uuid) -> Result<(), Self::Error> {
		let listeners_n = &mut self.chatrooms.get_mut(topic).unwrap().listeners_n;
		*listeners_n -= 1;

		if *listeners_n == 0 {
			self.chatrooms.remove(topic);
		}

		Ok(())
	}
}

impl MessagesListener {
	pub async fn new(
		db: &Database,
		publisher: Publisher<Uuid, Message, i64>,
	) -> sqlx::Result<Self> {
		let listener = PgListener::connect_with(&db.inner).await?;

		Ok(Self {
			db: listener,
			publisher,
			chatrooms: HashMap::new(),
		})
	}
	pub async fn run(mut self) -> anyhow::Result<()> {
		loop {
			select! {
				_ = self.publisher.drive(ChatroomTopicControl{ db: &mut self.db, chatrooms: &mut self.chatrooms }) => {},
				notification = self.db.try_recv() => {
					let notification = match notification? {
						Some(x) => x,
						None => {
							todo!();
						},
					};

					let chat_id: Uuid = notification.channel().strip_prefix("chat-").unwrap().parse().unwrap();

					#[derive(Clone, Debug, Deserialize)]
					struct NotificationPayload {
						id: Uuid,
						sequence_id: i64,
						user_id: Uuid,
						#[serde(default)]
						message: Option<String>,
						sent_at: DateTime<Local>,
					}

					let payload: NotificationPayload = match serde_json::from_str(notification.payload()) {
						Ok(x) => x,
						Err(e) => {
							bail!("couldnt parse notification payload ({}): {e}", notification.payload());
						},
					};

					let new_message;
					if let Some(message) = payload.message {
						new_message = Message {
							id: payload.id,
							sequence_id: payload.sequence_id,
							user_id: payload.user_id,
							message,
							sent_at: payload.sent_at
						};
					} else {
						// full message couldnt fit in the notification payload, gotta fetch it manually
						todo!()
					}

					self.publisher.publish(&chat_id, new_message).unwrap();
				}
			}
		}
	}
}

// struct ChatListener {
// 	updates: broadcast::Sender<Arc<Message>>,
// 	last_recv_id: i64,
// }

// impl ChatListener {
// 	async fn new(db: &mut PgListener, id: Uuid) -> sqlx::Result<Self> {
// 		db.listen(&format!("chat-{id}")).await?;

// 		// now we are already listening, so we can fetch the current last message seq id
// 		// and be sure that we are not gonna miss any since that one
// 		let id = sqlx::query_scalar!(
// 			r#"SELECT sequence_id
// 			FROM messages
// 			WHERE chatroom = $1
// 			ORDER BY sequence_id DESC
// 			LIMIT 1"#,
// 			id
// 		)
// 		.fetch_optional(db)
// 		.await?;

// 		Ok(Self {
// 			updates: broadcast::Sender::new(16),
// 			// id is None if there are no messages in that channel,
// 			// and they start from 0 so we have "received" -1
// 			last_recv_id: id.unwrap_or(-1),
// 		})
// 	}
// 	fn subscribe(&self) -> broadcast::Receiver<Arc<Message>> {
// 		self.updates.subscribe()
// 	}
// 	fn last_recv_id(&self) -> i64 {
// 		self.last_recv_id
// 	}
// }

// pub enum ControlMessage {
// 	SubscribeToChat {
// 		chatroom_id: Uuid,
// 		respond: oneshot::Sender<SubscribeToChatResponse>,
// 	},
// }

// #[derive(Debug)]
// pub struct SubscribeToChatResponse {
// 	// the sequence id of the last message that was in the chat
// 	// before starting to listen to it
// 	pub last_message_seq_id: i64,
// 	pub updates: broadcast::Receiver<Arc<Message>>,
// }
