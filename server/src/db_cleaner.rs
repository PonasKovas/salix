use crate::ServerState;
use anyhow::Result;
use sqlx::postgres::PgAdvisoryLock;
use std::time::Duration;
use tokio::{spawn, time::sleep};
use tracing::{debug, error};

const CLEAN_INTERVAL: u64 = 60; // in minutes
const PG_LOCK_NAME: &'static str = "DATABASE_CLEANUP";

pub async fn init_cleaner(mut state: ServerState) {
	spawn(async move {
		loop {
			if let Err(e) = clean(&mut state).await {
				error!("error cleaning database: {e}");
			}

			sleep(Duration::from_mins(CLEAN_INTERVAL)).await;
		}
	});
}

async fn clean(state: &mut ServerState) -> Result<()> {
	let mut transaction = state.db.transaction().await?;

	let lock_key = PgAdvisoryLock::new(PG_LOCK_NAME).key().as_bigint().unwrap();

	let locked = sqlx::query_scalar!(r#"SELECT pg_try_advisory_xact_lock($1)"#, lock_key)
		.fetch_one(transaction.as_mut())
		.await?
		.expect("pg_try_advisory_xact_lock should never return NULL");

	if !locked {
		// couldnt lock - already locked. someone else is cleaning rn. just skip
		debug!("Attempt to clean database but advisory lock already locked.");
		transaction.commit().await?;
		return Ok(());
	}

	let tables_to_clean = sqlx::query_scalar!(
		r#"SELECT table_name
	    FROM cleanup_log
	    WHERE last_cleaned_at + ('1 min'::interval * $1) <= NOW()
	    FOR UPDATE SKIP LOCKED"#,
		CLEAN_INTERVAL as f64
	)
	.fetch_all(transaction.as_mut())
	.await?;

	for table_name in tables_to_clean {
		match table_name.as_str() {
			"email_verifications" => {
				sqlx::query!("DELETE FROM email_verifications WHERE expires_at < NOW()")
					.execute(transaction.as_mut())
					.await?;
			}
			"registrations" => {
				sqlx::query!("DELETE FROM registrations WHERE expires_at < NOW()")
					.execute(transaction.as_mut())
					.await?;
			}
			"active_sessions" => {
				sqlx::query!("DELETE FROM active_sessions WHERE expires_at < NOW()")
					.execute(transaction.as_mut())
					.await?;
			}
			other => {
				error!("unknown table to be cleaned: {other}");
				continue;
			}
		}

		// update the last_cleaned_at timestamp
		sqlx::query!(
			"UPDATE cleanup_log SET last_cleaned_at = NOW() WHERE table_name = $1",
			table_name
		)
		.execute(transaction.as_mut())
		.await?;
	}

	transaction.commit().await?;

	Ok(())
}
