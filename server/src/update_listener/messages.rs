use crate::database::{Database, message::Message};
use ahash::{HashMap, HashMapExt};
use anyhow::{Context, bail};
use chrono::{DateTime, Local};
use futures::StreamExt;
use serde::Deserialize;
use sqlx::{
	PgPool,
	postgres::{PgListener, PgNotification},
};
use std::{collections::hash_map::Entry, convert::Infallible};
use tokio::{select, spawn};
use tokio_pubsub::{EventReactor, Publisher, PublisherHandle};
use tracing::error;
use uuid::Uuid;

pub struct MessagesListener {
	db: Database<PgListener>,
	chatrooms: HashMap<Uuid, ChatroomState>,
}

type MessagesPublisher = Publisher<Uuid, Message, ChatroomContext>;

struct ChatroomState {
	// will stop listening when it reaches 0
	listeners_n: u32,
	// the sequential ID of the last received message
	last_received_seq_id: i64,
}

pub struct ChatroomContext {
	/// guarantees that all messages AFTER this seq id will be
	/// delivered as long as you keep listening
	pub last_message_seq_id: i64,
}

pub async fn start(
	db: &Database<PgPool>,
) -> sqlx::Result<PublisherHandle<Uuid, Message, ChatroomContext>> {
	let publisher = Publisher::new();
	let handle = publisher.handle();

	let messages_listener = MessagesListener::new(db).await?;
	spawn(async move {
		if let Err(e) = messages_listener.run(publisher).await {
			error!("{e:?}");
		}
	});

	Ok(handle)
}

impl MessagesListener {
	pub async fn new(db: &Database<PgPool>) -> sqlx::Result<Self> {
		let listener = PgListener::connect_with(&db.inner).await?;

		Ok(Self {
			db: Database::new(listener),
			chatrooms: HashMap::new(),
		})
	}
	// publisher has to be separate from Self, because drive() borrows self, and we need self again to finish it
	pub async fn run(mut self, mut publisher: MessagesPublisher) -> anyhow::Result<()> {
		loop {
			select! {
				driver = publisher.drive() => {
					struct Reactor<'a>(&'a mut MessagesListener);

					impl<'a> EventReactor<Uuid, ChatroomContext, Infallible> for Reactor<'a> {
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

					driver.finish(Reactor(&mut self)).await?;
				},
				notification = self.db.try_recv() => {
					self.handle_notification(&mut publisher, notification?).await.context("handle notification")?;
				}
			}
		}
	}
	async fn handle_notification(
		&mut self,
		publisher: &mut MessagesPublisher,
		notification: Option<PgNotification>,
	) -> anyhow::Result<()> {
		let notification = match notification {
			Some(x) => x,
			None => {
				// disrupted connection, fetch all messages since last received and continue
				self.on_db_conn_disruption(publisher)
					.await
					.context("handle conn disruption")?;

				return Ok(());
			}
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
				bail!(
					"couldnt parse notification payload ({}): {e}",
					notification.payload()
				);
			}
		};

		let new_message;
		if let Some(message) = payload.message {
			new_message = Message {
				id: payload.id,
				chatroom: chat_id,
				sequence_id: payload.sequence_id,
				user_id: payload.user_id,
				message,
				sent_at: payload.sent_at,
			};
		} else {
			// full message couldnt fit in the notification payload, gotta fetch it manually
			new_message = self.db.message_by_id(payload.id).await?.unwrap();
		}

		self.chatrooms
			.get_mut(&chat_id)
			.unwrap()
			.last_received_seq_id = payload.sequence_id;

		publisher.publish(&chat_id, new_message).unwrap();

		Ok(())
	}
	// gets called when the database connection is disrupted and there might have been missed new messages
	async fn on_db_conn_disruption(
		&mut self,
		publisher: &mut MessagesPublisher,
	) -> sqlx::Result<()> {
		for (chat_id, chatroom) in &mut self.chatrooms {
			let fetch_since = chatroom.last_received_seq_id;

			let mut msg_stream = self.db.messages_by_seq_id(chat_id, fetch_since..);

			while let Some(msg) = msg_stream.next().await {
				let msg = msg?;

				chatroom.last_received_seq_id = msg.sequence_id;
				publisher.publish(chat_id, msg).unwrap();
			}
		}

		Ok(())
	}
	async fn on_subscribe(&mut self, topic: &Uuid) -> sqlx::Result<ChatroomContext> {
		let chatroom_data = match self.chatrooms.entry(topic.clone()) {
			Entry::Occupied(occupied_entry) => occupied_entry.into_mut(),
			Entry::Vacant(vacant_entry) => {
				let new_state = start_chat_listen(&mut self.db, topic).await?;

				vacant_entry.insert(new_state)
			}
		};
		chatroom_data.listeners_n += 1;

		Ok(ChatroomContext {
			last_message_seq_id: chatroom_data.last_received_seq_id,
		})
	}
	async fn on_unsubscribe(&mut self, topic: &Uuid) -> Result<(), sqlx::Error> {
		let listeners_n = &mut self.chatrooms.get_mut(topic).unwrap().listeners_n;
		*listeners_n -= 1;

		if *listeners_n == 0 {
			self.db.unlisten(&channel_name_from_uuid(topic)).await?;
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

async fn start_chat_listen(
	db: &mut Database<PgListener>,
	topic: &Uuid,
) -> sqlx::Result<ChatroomState> {
	db.listen(&channel_name_from_uuid(topic)).await?;

	// now we are already listening, so we can fetch the current last message seq id
	// and be sure that we are not gonna miss any since that one
	let last_seq_id = db.fetch_last_message_seq_id(topic).await?;

	Ok(ChatroomState {
		listeners_n: 0,
		last_received_seq_id: last_seq_id,
	})
}
