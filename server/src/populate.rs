use crate::database::Database;
use sqlx::{PgPool, query};
use uuid::Uuid;

pub async fn populate(db: Database<PgPool>) -> anyhow::Result<()> {
	let db = db.inner;

	query!(
		r#"INSERT INTO users (id, username, email, password) VALUES ($1, 'Alex', 'alex@email.com', 'test')"#,
		Uuid::from_u128(1)
	).execute(&db).await?;
	query!(
		r#"INSERT INTO users (id, username, email, password) VALUES ($1, 'Bob', 'bob@email.com', 'test')"#,
		Uuid::from_u128(2)
	).execute(&db).await?;

	query!(
		r#"INSERT INTO active_sessions (token, user_id, expires_at) VALUES ($1, $2, NOW() + ('10 years'::interval))"#,
		Uuid::from_u128(3),
		Uuid::from_u128(1),
	)
	.execute(&db)
	.await?;
	query!(
		r#"INSERT INTO active_sessions (token, user_id, expires_at) VALUES ($1, $2, NOW() + ('10 years'::interval))"#,
		Uuid::from_u128(4),
		Uuid::from_u128(2),
	)
	.execute(&db)
	.await?;

	query!(
		r#"INSERT INTO chatrooms (id, name) VALUES ($1, 'test room')"#,
		Uuid::from_u128(5),
	)
	.execute(&db)
	.await?;

	Ok(())
}
