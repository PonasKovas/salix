use crate::{cmd_args::Args, config::Config};
use sqlx::{Executor, PgPool, Postgres};

pub mod active_sessions;
pub mod message;
pub mod user;

#[derive(Clone, Debug)]
pub struct Database {
	pub inner: PgPool,
}

impl Database {
	pub async fn init(config: &Config, args: &Args) -> sqlx::Result<Self> {
		let pool = PgPool::connect(&config.database_url).await?;

		if !args.no_migrate {
			sqlx::migrate!().run(&pool).await?;
		}

		Ok(Self { inner: pool })
	}
}
