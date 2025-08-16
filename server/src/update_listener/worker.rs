use crate::db::{Database, message::Message};
use ahash::{HashMap, HashMapExt};
use sqlx::postgres::PgListener;
use std::{collections::hash_map::Entry, sync::Arc};
use tokio::{
	select,
	sync::{broadcast, mpsc, oneshot},
};
use uuid::Uuid;

pub struct ListenerWorker {
	db: PgListener,
	chat_messages: HashMap<Uuid, ChatListener>,
}

struct ChatListener {
	updates: broadcast::Sender<Arc<Message>>,
	last_recv_id: i64,
}

impl ChatListener {
	async fn new(db: &mut PgListener, id: Uuid) -> sqlx::Result<Self> {
		db.listen(&format!("chat-{id}")).await?;

		// now we are already listening, so we can fetch the current last message seq id
		// and be sure that we are not gonna miss any since that one
		let id = sqlx::query_scalar!(
			r#"SELECT sequence_id
			FROM messages
			WHERE chatroom = $1
			ORDER BY sequence_id DESC
			LIMIT 1"#,
			id
		)
		.fetch_optional(db)
		.await?;

		Ok(Self {
			updates: broadcast::Sender::new(16),
			// id is None if there are no messages in that channel,
			// and they start from 0 so we have "received" -1
			last_recv_id: id.unwrap_or(-1),
		})
	}
	fn subscribe(&self) -> broadcast::Receiver<Arc<Message>> {
		self.updates.subscribe()
	}
	fn last_recv_id(&self) -> i64 {
		self.last_recv_id
	}
}

pub enum ControlMessage {
	SubscribeToChat {
		chatroom_id: Uuid,
		respond: oneshot::Sender<SubscribeToChatResponse>,
	},
}

#[derive(Debug)]
pub struct SubscribeToChatResponse {
	// the sequence id of the last message that was in the chat
	// before starting to listen to it
	pub last_message_seq_id: i64,
	pub updates: broadcast::Receiver<Arc<Message>>,
}

impl ListenerWorker {
	pub async fn new(db: &Database) -> sqlx::Result<Self> {
		let listener = PgListener::connect_with(&db.inner).await?;

		Ok(Self {
			chat_messages: HashMap::new(),
			db: listener,
		})
	}
	pub async fn start(mut self, mut control: mpsc::Receiver<ControlMessage>) -> sqlx::Result<()> {
		loop {
			select! {
				control_request = control.recv() => self.handle_control(control_request).await?,
				// notification = self.db.inner.try_recv() => {
					// let notification = match notification? {
					// 	Some(x) => x,
					// 	None => {
					// 		send_broadcast_with_cleanup(&mut updates, UpdateMessage::Disrupted);
					// 		continue;
					// 	},
					// };

					// #[derive(Clone, Debug, Deserialize)]
					// struct NotificationPayload {
					// 	id: Uuid,
					// 	sequence_id: i64,
					// 	user_id: Uuid,
					// 	#[serde(default)]
					// 	message: Option<String>,
					// 	sent_at: DateTime<Local>,
					// }

					// let payload: NotificationPayload = match serde_json::from_str(notification.payload()) {
					// 	Ok(x) => x,
					// 	Err(e) => {
					// 		error!("couldnt parse notification payload ({}): {e}", notification.payload());
					// 		continue;
					// 	},
					// };

					// let new_message;
					// if let Some(message) = payload.message {
					// 	new_message = Message{
					// 		id: payload.id,
					// 		sequence_id: payload.sequence_id,
					// 		user_id: payload.user_id,
					// 		message,
					// 		sent_at: payload.sent_at
					// 	};
					// } else {
					// 	// full message couldnt fit in the notification payload, gotta fetch it manually
					// 	new_message = db.message_by_id(payload.id).await?.ok_or(sqlx::Error::RowNotFound)?;
					// }

					// send_broadcast_with_cleanup(&mut updates, new_message);
				// }
			}
		}
	}
	async fn handle_control(
		&mut self,
		control_request: Option<ControlMessage>,
	) -> sqlx::Result<()> {
		let control_request = match control_request {
			Some(x) => x,
			None => return Ok(()), // all control senders dropped
		};

		match control_request {
			ControlMessage::SubscribeToChat {
				chatroom_id,
				respond,
			} => {
				let chat_listener = match self.chat_messages.entry(chatroom_id.clone()) {
					Entry::Occupied(x) => x,
					Entry::Vacant(vacant) => {
						vacant.insert_entry(ChatListener::new(&mut self.db, chatroom_id).await?)
					}
				};

				respond
					.send(SubscribeToChatResponse {
						last_message_seq_id: chat_listener.get().last_recv_id(),
						updates: chat_listener.get().subscribe(),
					})
					.unwrap();
			}
		}

		Ok(())
	}
}

fn send_broadcast_with_cleanup(sender: &mut broadcast::Sender<Arc<Message>>, msg: Message) {
	let _ = sender.send(Arc::new(msg));
	// todo when multiple chatrooms, if no subscribers left (send errs)
	// stop listening altogether
}
