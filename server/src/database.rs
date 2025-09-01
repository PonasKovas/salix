use crate::{cmd_args::Args, config::Config};
use sqlx::{Executor, PgPool, Postgres, postgres::PgListener};
use std::ops::{Deref, DerefMut};

pub mod active_sessions;
pub mod message;
pub mod user;

#[derive(Clone, Debug)]
pub struct Database<D> {
	pub inner: D,
}

impl Database<PgPool> {
	pub async fn init(config: &Config, args: &Args) -> sqlx::Result<Self> {
		let pool = PgPool::connect(&config.database_url).await?;

		if !args.no_migrate {
			sqlx::migrate!().run(&pool).await?;
		}

		Ok(Self { inner: pool })
	}
}
impl<D: ExecutorHack> Database<D> {
	pub fn new(inner: D) -> Self {
		Self { inner }
	}
	fn as_executor(&mut self) -> impl Executor<'_, Database = Postgres> {
		self.inner.as_executor()
	}
}

impl<D> Deref for Database<D> {
	type Target = D;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}
impl<D> DerefMut for Database<D> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}

pub trait ExecutorHack {
	type Executor<'a>: Executor<'a, Database = Postgres>
	where
		Self: 'a;

	fn as_executor<'a>(&'a mut self) -> Self::Executor<'a>;
}

impl ExecutorHack for PgPool {
	type Executor<'a> = &'a PgPool;

	fn as_executor<'a>(&'a mut self) -> Self::Executor<'a> {
		self
	}
}
impl ExecutorHack for PgListener {
	type Executor<'a> = &'a mut PgListener;

	fn as_executor<'a>(&'a mut self) -> Self::Executor<'a> {
		self
	}
}
