use super::{Database, ExecutorHack};
use uuid::Uuid;

pub struct Registration {
	pub email: String,
}

impl<D: ExecutorHack> Database<D> {
	pub async fn insert_registration(
		&mut self,
		id: Uuid,
		email: &str,
		lifetime_minutes: u32,
	) -> sqlx::Result<()> {
		sqlx::query!(
			r#"
			INSERT INTO registrations (id, email, expires_at)
			VALUES ($1, $2, NOW() + ('1 min'::interval * $3))"#,
			id,
			email,
			lifetime_minutes as i64
		)
		.execute(self.as_executor())
		.await
		.map(|_| ())
	}
	pub async fn get_registration(&mut self, id: Uuid) -> sqlx::Result<Option<Registration>> {
		sqlx::query_as!(
			Registration,
			r#"SELECT email
			FROM registrations
			WHERE id = $1 AND expires_at > NOW()
			FOR UPDATE"#,
			id
		)
		.fetch_optional(self.as_executor())
		.await
	}
	pub async fn remove_registration(&mut self, id: Uuid) -> sqlx::Result<()> {
		sqlx::query_as!(
			Registration,
			r#"DELETE FROM registrations
    		WHERE id = $1"#,
			id
		)
		.execute(self.as_executor())
		.await
		.map(|_| ())
	}
}
