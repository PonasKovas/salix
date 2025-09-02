use crate::database::Database;
use sqlx::{PgPool, query};
use uuid::Uuid;

pub const USER_A_ID: Uuid = Uuid::from_u128(1);
pub const USER_B_ID: Uuid = Uuid::from_u128(2);
pub const USER_A_TOKEN: Uuid = Uuid::from_u128(3);
pub const USER_B_TOKEN: Uuid = Uuid::from_u128(4);
pub const CHAT_ID: Uuid = Uuid::from_u128(5);

pub async fn populate(db: &Database<PgPool>) -> anyhow::Result<()> {
	let db = &db.inner;

	query!(
		r#"INSERT INTO users (id, username, email, password) VALUES ($1, 'Alex', 'alex@email.com', 'test') ON CONFLICT (id) DO NOTHING"#,
		USER_A_ID
	).execute(db).await?;
	query!(
		r#"INSERT INTO users (id, username, email, password) VALUES ($1, 'Bob', 'bob@email.com', 'test') ON CONFLICT (id) DO NOTHING"#,
		USER_B_ID
	).execute(db).await?;

	query!(
		r#"INSERT INTO active_sessions (token, user_id, expires_at) VALUES ($1, $2, NOW() + ('10 years'::interval)) ON CONFLICT (token) DO NOTHING"#,
		USER_A_TOKEN,
		USER_A_ID,
	)
	.execute(db)
	.await?;
	query!(
		r#"INSERT INTO active_sessions (token, user_id, expires_at) VALUES ($1, $2, NOW() + ('10 years'::interval)) ON CONFLICT (token) DO NOTHING"#,
		USER_B_TOKEN,
		USER_B_ID,
	)
	.execute(db)
	.await?;

	query!(
		r#"INSERT INTO chatrooms (id, name) VALUES ($1, 'test room') ON CONFLICT (id) DO NOTHING"#,
		CHAT_ID,
	)
	.execute(db)
	.await?;

	Ok(())
}
