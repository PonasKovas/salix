use super::{Database, ExecutorHack};
use chrono::{DateTime, Local};
use futures::Stream;
use std::{
	ops::{Bound, RangeBounds},
	pin::Pin,
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Message {
	pub id: Uuid,
	pub sequence_id: i64,
	pub user_id: Uuid,
	pub message: String,
	pub sent_at: DateTime<Local>,
}

impl<D: ExecutorHack> Database<D> {
	pub async fn message_by_id(&mut self, id: Uuid) -> sqlx::Result<Option<Message>> {
		sqlx::query_as!(
			Message,
			r#"SELECT id, sequence_id, user_id, message, sent_at
			FROM messages
			WHERE id = $1"#,
			id,
		)
		.fetch_optional(self.as_executor())
		.await
	}
	pub fn messages_by_seq_id(
		&mut self,
		seq_range: impl RangeBounds<i64>,
	) -> Pin<Box<dyn Stream<Item = sqlx::Result<Message>> + Send + '_>> {
		let start = match seq_range.start_bound() {
			Bound::Included(s) => Some(*s),
			Bound::Excluded(s) => Some(*s + 1),
			Bound::Unbounded => None,
		};
		let end = match seq_range.end_bound() {
			Bound::Included(e) => Some(*e),
			Bound::Excluded(e) => Some(*e - 1),
			Bound::Unbounded => None,
		};

		sqlx::query_as!(
			Message,
			r#"SELECT id, sequence_id, user_id, message, sent_at
			FROM messages
			WHERE
	            ($1::BIGINT IS NULL OR sequence_id >= $1)
	        AND
	            ($2::BIGINT IS NULL OR sequence_id <= $2)
	        ORDER BY sequence_id ASC"#,
			start,
			end
		)
		.fetch(self.as_executor())
	}
	/// Returns (message uuid, sequential id)
	pub async fn insert_message(
		&mut self,
		chatroom_id: Uuid,
		user_id: Uuid,
		message: &str,
	) -> sqlx::Result<(Uuid, i64)> {
		let msg_id = Uuid::now_v7();

		sqlx::query_scalar!(
			r#"SELECT add_message($1, $2, $3, $4)"#,
			msg_id,
			chatroom_id,
			user_id,
			message
		)
		.fetch_one(self.as_executor())
		.await
		.map(|seq_id| (msg_id, seq_id.unwrap()))
	}
	pub async fn fetch_last_message_seq_id(&mut self, chatroom_id: &Uuid) -> sqlx::Result<i64> {
		sqlx::query_scalar!(
			r#"SELECT sequence_id
			FROM messages
			WHERE chatroom = $1
			ORDER BY sequence_id DESC
			LIMIT 1"#,
			chatroom_id
		)
		.fetch_optional(self.as_executor())
		.await
		.map(|opt| opt.unwrap_or(-1))
	}
}

#[derive(Clone, Debug)]
pub struct MessageInsert {
	pub id: Uuid,
	pub user_id: Uuid,
	pub message: String,
}
