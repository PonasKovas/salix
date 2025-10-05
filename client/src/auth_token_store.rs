use crate::auth::AuthToken;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StoredAuthTokenError {
	#[error("error accessing the keyring")]
	Keyring(#[from] keyring::Error),
	#[error("invalid uuid")]
	InvalidUuid(#[from] uuid::Error),
}

pub fn get_stored_auth_token() -> Result<Option<AuthToken>, StoredAuthTokenError> {
	let secret = match keyring::Entry::new("chat.salix", "auth_token")?.get_password() {
		Ok(x) => x,
		Err(keyring::Error::NoEntry) => return Ok(None),
		Err(e) => return Err(e.into()),
	};

	let uuid = Uuid::parse_str(&secret)?;

	Ok(Some(AuthToken(uuid)))
}

pub fn store_auth_token(token: &AuthToken) -> Result<(), StoredAuthTokenError> {
	keyring::Entry::new("chat.salix", "auth_token")?.set_password(&token.0.to_string())?;

	Ok(())
}
