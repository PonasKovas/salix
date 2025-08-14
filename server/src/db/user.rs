use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct User {
	pub id: Uuid,
	pub username: String,
	pub email: String,
	pub password: String,
}

impl User {
	pub fn by_auth_token(token: Uuid) -> map_type!('static, User) {
		sqlx::query_as!(
			User,
			r#"SELECT u.id, u.username, u.email, u.password
			FROM active_sessions JOIN users AS u ON active_sessions.user_id = u.id
			WHERE active_sessions.token = $1"#,
			token,
		)
	}
	pub fn by_id(id: Uuid) -> map_type!('static, User) {
		sqlx::query_as!(
			User,
			r#"SELECT id, username, email, password
			FROM users
			WHERE id = $1"#,
			id,
		)
	}
	pub fn by_username<'a>(username: &'a str) -> map_type!('a, User) {
		sqlx::query_as!(
			User,
			r#"SELECT id, username, email, password
			FROM users
			WHERE username = $1"#,
			username,
		)
	}
	pub fn by_email<'a>(email: &'a str) -> map_type!('a, User) {
		sqlx::query_as!(
			User,
			r#"SELECT id, username, email, password
			FROM users
			WHERE email = $1"#,
			email,
		)
	}
}
