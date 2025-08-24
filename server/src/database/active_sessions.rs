use super::{Database, ExecutorHack};
use uuid::Uuid;

impl<D: ExecutorHack> Database<D> {
	pub async fn insert_active_session(
		&mut self,
		user_id: Uuid,
		lifetime_hours: u32,
	) -> sqlx::Result<Uuid> {
		let session_token = Uuid::now_v7();

		sqlx::query!(
			r#"INSERT INTO active_sessions (token, user_id, expires_at)
			VALUES ($1, $2, NOW() + ('1 hour'::interval * $3))"#,
			session_token,
			user_id,
			lifetime_hours as i64
		)
		.execute(self.as_executor())
		.await
		.map(|_| session_token)
	}
}
