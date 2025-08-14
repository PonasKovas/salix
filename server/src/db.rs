macro_rules! map_type {
    ($lifetime:lifetime, $return_type:ident) => {
        sqlx::query::Map<
			$lifetime,
			sqlx::Postgres,
			impl FnMut(<sqlx::Postgres as sqlx::Database>::Row) -> Result<$return_type, sqlx::Error> + Send,
			impl Send + sqlx::IntoArguments<$lifetime, sqlx::Postgres>,
		>
    };
}

mod message;
mod user;

pub use message::Message;
pub use user::User;
