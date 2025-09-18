use protocol::auth::v1 as auth;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum LoginError {
	#[error(transparent)]
	Reqwest(#[from] reqwest::Error),
	#[error(transparent)]
	Api(#[from] auth::Error),
}

pub async fn login(email: String, password: String) -> Result<Uuid, LoginError> {
	let client = reqwest::Client::new();

	let data = auth::LoginRequest { email, password };

	let res = client
		.post("http://localhost:3000/auth/v1/login")
		.json(&data)
		.send()
		.await?;

	if !res.status().is_success() {
		let error = res.json::<auth::Error>().await?;

		return Err(LoginError::Api(error));
	}

	let response_data = res.json::<auth::LoginSuccess>().await?;

	Ok(response_data.auth_token)
}

pub async fn register(email: String, password: String) -> Result<Uuid, LoginError> {
	let client = reqwest::Client::new();

	let data = auth::LoginRequest { email, password };

	let res = client
		.post("http://localhost:3000/auth/v1/login")
		.json(&data)
		.send()
		.await?;

	if !res.status().is_success() {
		let error = res.json::<auth::Error>().await?;

		return Err(LoginError::Api(error));
	}

	let response_data = res.json::<auth::LoginSuccess>().await?;

	Ok(response_data.auth_token)
}
