use super::{Database, ExecutorHack};
use thiserror::Error;

const MAX_VERIFICATION_ATTEMPTS: i16 = 3;

#[derive(Debug, Error)]
#[error("email already in the process of being verified")]
pub struct EmailAlreadyAdded;

#[derive(Debug, Error)]
pub enum VerifyEmailError {
	#[error("code is incorrect")]
	IncorrectCode,
	#[error("too many attempts")]
	TooManyAttempts,
	#[error("invalid")]
	Invalid,
}

impl<D: ExecutorHack> Database<D> {
	pub async fn insert_email_verification(
		&mut self,
		email: &str,
		link_token_hash: &str,
		code: u16,
		lifetime_minutes: u32,
	) -> sqlx::Result<Result<(), EmailAlreadyAdded>> {
		match sqlx::query!(
			r#"
			INSERT INTO email_verifications (email, link_token_hash, code, expires_at)
			VALUES ($1, $2, $3, NOW() + ('1 min'::interval * $4))
			ON CONFLICT (email)
			DO UPDATE SET
			    link_token_hash = $2,
			    code = $3,
			    expires_at = NOW() + ('1 min'::interval * $4),
			    attempts = 0
			WHERE
			    email_verifications.expires_at < NOW()"#,
			email,
			link_token_hash,
			code as i16,
			lifetime_minutes as i64
		)
		.execute(self.as_executor())
		.await
		{
			Ok(_) => Ok(Ok(())),
			Err(sqlx::Error::Database(db_err)) => {
				if db_err.is_unique_violation()
					&& db_err.constraint() == Some("email_verifications_email_key")
				{
					Ok(Err(EmailAlreadyAdded))
				} else {
					Err(sqlx::Error::Database(db_err))
				}
			}
			Err(e) => Err(e),
		}
	}
	pub async fn get_email_verification_code(
		&mut self,
		link_token_hash: &str,
	) -> sqlx::Result<Option<u32>> {
		sqlx::query_scalar!(
			r#"SELECT code
			FROM email_verifications
			WHERE link_token_hash = $1"#,
			link_token_hash,
		)
		.fetch_optional(self.as_executor())
		.await
		.map(|i| i.map(|i| i as u32))
	}
	pub async fn verify_email(
		&mut self,
		email: &str,
		code: u32,
	) -> sqlx::Result<Result<(), VerifyEmailError>> {
		let mut transaction = self.transaction().await?;

		let res = match sqlx::query!(
			r#"UPDATE email_verifications
			SET attempts = attempts + 1
			WHERE email = $1 AND expires_at > NOW()
			RETURNING code, attempts"#,
			email
		)
		.fetch_optional(transaction.as_executor())
		.await?
		{
			Some(res) => res,
			None => {
				return Ok(Err(VerifyEmailError::Invalid));
			}
		};

		if res.attempts >= MAX_VERIFICATION_ATTEMPTS {
			transaction.commit().await?;
			return Ok(Err(VerifyEmailError::TooManyAttempts));
		}

		if res.code as u32 != code {
			transaction.commit().await?;
			return Ok(Err(VerifyEmailError::IncorrectCode));
		}

		sqlx::query!(r#"DELETE FROM email_verifications WHERE email = $1"#, email)
			.execute(transaction.as_executor())
			.await?;

		transaction.commit().await?;

		Ok(Ok(()))
	}
}
