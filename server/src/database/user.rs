use super::{Database, ExecutorHack};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct User {
	pub id: Uuid,
	pub username: String,
	pub email: String,
	pub password: String,
	pub last_account_reminder_sent: DateTime<Utc>,
}

pub struct UsernameConflict;

impl<D: ExecutorHack> Database<D> {
	pub async fn user_by_auth_token(&mut self, token: Uuid) -> sqlx::Result<Option<User>> {
		sqlx::query_as!(
			User,
			r#"SELECT u.id, u.username, u.email, u.password, u.last_account_reminder_sent
			FROM active_sessions JOIN users AS u ON active_sessions.user_id = u.id
			WHERE active_sessions.token = $1 AND active_sessions.expires_at > NOW()"#,
			token,
		)
		.fetch_optional(self.as_executor())
		.await
	}
	pub async fn user_by_id(&mut self, id: Uuid) -> sqlx::Result<Option<User>> {
		sqlx::query_as!(
			User,
			r#"SELECT id, username, email, password, last_account_reminder_sent
			FROM users
			WHERE id = $1"#,
			id,
		)
		.fetch_optional(self.as_executor())
		.await
	}
	pub async fn user_by_username(&mut self, username: &str) -> sqlx::Result<Option<User>> {
		sqlx::query_as!(
			User,
			r#"SELECT id, username, email, password, last_account_reminder_sent
			FROM users
			WHERE username = $1"#,
			username,
		)
		.fetch_optional(self.as_executor())
		.await
	}
	pub async fn user_by_email(&mut self, email: &str) -> sqlx::Result<Option<User>> {
		sqlx::query_as!(
			User,
			r#"SELECT id, username, email, password, last_account_reminder_sent
			FROM users
			WHERE email = $1"#,
			email,
		)
		.fetch_optional(self.as_executor())
		.await
	}
	pub async fn insert_user(
		&mut self,
		username: &str,
		email: &str,
		password: &str,
	) -> sqlx::Result<Result<Uuid, UsernameConflict>> {
		let user_id = Uuid::now_v7();

		match sqlx::query!(
			r#"INSERT INTO users (id, username, email, password) VALUES ($1, $2, $3, $4)"#,
			user_id,
			username,
			email,
			password
		)
		.execute(self.as_executor())
		.await
		{
			Ok(_) => Ok(Ok(user_id)),
			Err(sqlx::Error::Database(db_err)) => {
				if db_err.is_unique_violation() && db_err.constraint() == Some("users_username_key")
				{
					Ok(Err(UsernameConflict))
				} else {
					Err(sqlx::Error::Database(db_err))
				}
			}
			Err(e) => Err(e),
		}
	}
}
