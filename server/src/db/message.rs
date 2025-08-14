use chrono::{DateTime, Local};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Message {
	pub id: Uuid,
	pub sequence_id: i64,
	pub user_id: Uuid,
	pub message: String,
	pub sent_at: DateTime<Local>,
}

impl Message {
	pub fn by_id(id: Uuid) -> map_type!('static, Message) {
		sqlx::query_as!(
			Message,
			r#"SELECT id, sequence_id, user_id, message, sent_at
			FROM messages
			WHERE id = $1"#,
			id,
		)
	}
}
