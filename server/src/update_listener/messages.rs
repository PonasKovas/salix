use crate::database::{Database, message::Message};
use ahash::{HashMap, HashMapExt};
use always_send::FutureExt;
use anyhow::bail;
use chrono::{DateTime, Local};
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgListener};
use std::{collections::hash_map::Entry, convert::Infallible};
use tokio::select;
use tokio_pubsub::{EventReactor, Publisher};
use uuid::Uuid;

pub type MessagesPublisher = Publisher<Uuid, Message, ChatroomContext>;

pub struct MessagesListener {
	db: Database<PgListener>,
	chatrooms: HashMap<Uuid, ChatroomState>,
}

struct ChatroomState {
	listeners_n: u32,
	last_message_seq_id: i64,
}

pub struct ChatroomContext {
	last_message_seq_id: i64,
}

struct MessagesReactor<'a>(&'a mut MessagesListener);
impl<'a> EventReactor<Uuid, ChatroomContext, Infallible> for MessagesReactor<'a> {
	type Error = sqlx::Error;

	async fn on_subscribe(
		&mut self,
		topic: &Uuid,
	) -> Result<Result<ChatroomContext, Infallible>, Self::Error> {
		self.0.on_subscribe(topic).await.map(Ok)
	}
	async fn on_unsubscribe(&mut self, topic: &Uuid) -> Result<(), Self::Error> {
		self.0.on_unsubscribe(topic).await
	}
}

impl MessagesListener {
	pub async fn new(db: &Database<PgPool>) -> sqlx::Result<Self> {
		let listener = PgListener::connect_with(&db.inner).await?;

		Ok(Self {
			db: Database::new(listener),
			chatrooms: HashMap::new(),
		})
	}
	pub async fn run(mut self, mut publisher: MessagesPublisher) -> anyhow::Result<()> {
		loop {
			select! {
				driver = publisher.drive() => {
					driver.finish(MessagesReactor(&mut self)).always_send().await?;
				},
				notification = self.db.try_recv() => {
					let notification = match notification? {
						Some(x) => x,
						None => {
							// disrupted connection, fetch all messages since last received and continue
							todo!();
						},
					};

					let chat_id: Uuid = uuid_from_channel_name(notification.channel());

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
						new_message = self.db.message_by_id(payload.id).await?.unwrap();
					}

					publisher.publish(&chat_id, new_message).unwrap();
				}
			}
		}
	}
	async fn on_subscribe(&mut self, topic: &Uuid) -> Result<ChatroomContext, sqlx::Error> {
		let chatroom_data = match self.chatrooms.entry(topic.clone()) {
			Entry::Occupied(occupied_entry) => occupied_entry.into_mut(),
			Entry::Vacant(vacant_entry) => {
				self.db
					.listen(&channel_name_from_uuid(topic))
					.always_send()
					.await?;

				// now we are already listening, so we can fetch the current last message seq id
				// and be sure that we are not gonna miss any since that one
				let last_seq_id = self
					.db
					.fetch_last_message_seq_id(topic)
					.always_send()
					.await?;

				vacant_entry.insert(ChatroomState {
					listeners_n: 0,
					last_message_seq_id: last_seq_id,
				})
			}
		};
		chatroom_data.listeners_n += 1;

		Ok(ChatroomContext {
			last_message_seq_id: chatroom_data.last_message_seq_id,
		})
	}
	async fn on_unsubscribe(&mut self, topic: &Uuid) -> Result<(), sqlx::Error> {
		let listeners_n = &mut self.chatrooms.get_mut(topic).unwrap().listeners_n;
		*listeners_n -= 1;

		if *listeners_n == 0 {
			self.db
				.unlisten(&channel_name_from_uuid(topic))
				.always_send()
				.await?;
			self.chatrooms.remove(topic);
		}

		Ok(())
	}
}

fn channel_name_from_uuid(uuid: &Uuid) -> String {
	format!("chat-{uuid}")
}
fn uuid_from_channel_name(name: &str) -> Uuid {
	name.strip_prefix("chat-").unwrap().parse().unwrap()
}
