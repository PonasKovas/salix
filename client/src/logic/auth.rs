use protocol::auth::Request;
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

const BASE_API_URL: &'static str = "http://localhost:3000/auth/v1";

async fn make_request<R: Request>(req: &R) -> Result<R::Response, LoginError> {
	let url = format!("{BASE_API_URL}{}", R::PATH);

	let client = reqwest::Client::new();

	let res = client.post(url).json(req).send().await?;

	if !res.status().is_success() {
		let error = res.json::<auth::Error>().await?;

		return Err(LoginError::Api(error));
	}

	Ok(res.json::<R::Response>().await?)
}

pub async fn login(email: String, password: String) -> Result<Uuid, LoginError> {
	make_request(&auth::LoginRequest { email, password })
		.await
		.map(|success| success.auth_token)
}

pub async fn start_verify_email(email: String) -> Result<(), LoginError> {
	make_request(&auth::StartEmailVerifyRequest { email }).await
}

/// returns the registration id
pub async fn verify_email(email: String, code: u32) -> Result<Uuid, LoginError> {
	make_request(&auth::VerifyEmailRequest { email, code })
		.await
		.map(|response| response.registration_id)
}

/// returns an auth token
pub async fn finalize(
	registration_id: Uuid,
	username: String,
	password: String,
) -> Result<Uuid, LoginError> {
	make_request(&auth::FinalizeNewAccountRequest {
		registration_id,
		username,
		password,
	})
	.await
	.map(|response| response.auth_token)
}
