use crate::InnerClient;
use email_address::EmailAddress;
use protocol::auth::Request;
use protocol::auth::v1 as auth;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Auth {
	pub(super) client: Arc<InnerClient>,
}

#[derive(Debug, Clone)]
pub struct AuthToken(pub(crate) Uuid);

#[derive(Debug, Error)]
pub enum Error {
	#[error("error making the request")]
	Request(#[from] reqwest::Error),
	#[error(transparent)]
	Api(#[from] auth::Error),
	/// for verifying an email
	#[error("too many attempts")]
	TooManyAttempts,
	#[error("invalid email address")]
	InvalidEmail,
}

impl Auth {
	pub async fn login(&self, email: String, password: String) -> Result<AuthToken, Error> {
		make_request(&self.client, &auth::LoginRequest { email, password })
			.await
			.map(|success| AuthToken(success.auth_token))
	}
	pub async fn register(&self, email: String) -> Result<EmailVerifier, Error> {
		if !EmailAddress::is_valid(&email) {
			return Err(Error::InvalidEmail);
		}

		let request = auth::StartEmailVerifyRequest { email };
		make_request(&self.client, &request).await?;

		Ok(EmailVerifier {
			client: Arc::clone(&self.client),
			email: Some(request.email),
			attempts: 0,
		})
	}
}

#[derive(Debug, Clone)]
pub struct EmailVerifier {
	client: Arc<InnerClient>,
	email: Option<String>,
	attempts: u32,
}

impl EmailVerifier {
	pub async fn verify(&mut self, code: u32) -> Result<CreateAccount, Error> {
		let email = match self.email.take() {
			Some(x) => x,
			None => panic!("already verified"),
		};

		// the server would return "IncorrectCode" so we keep count on the client side too for a more user friendly error
		// plus we're saving the API call by returning it instantly
		if self.attempts >= 3 {
			return Err(Error::TooManyAttempts);
		}
		self.attempts += 1;

		let request = auth::VerifyEmailRequest { email, code };

		let registration_id = match make_request(&self.client, &request)
			.await
			.map(|response| response.registration_id)
		{
			Ok(x) => x,
			Err(e) => {
				self.email = Some(request.email);
				return Err(e);
			}
		};

		Ok(CreateAccount {
			client: Arc::clone(&self.client),
			registration_id,
		})
	}
}

#[derive(Debug, Clone)]
pub struct CreateAccount {
	client: Arc<InnerClient>,
	registration_id: Uuid,
}

impl CreateAccount {
	pub async fn finalize(&self, username: String, password: String) -> Result<AuthToken, Error> {
		let response = make_request(
			&self.client,
			&auth::FinalizeNewAccountRequest {
				registration_id: self.registration_id,
				username,
				password,
			},
		)
		.await?;

		Ok(AuthToken(response.auth_token))
	}
}

async fn make_request<R: Request>(client: &InnerClient, req: &R) -> Result<R::Response, Error> {
	let api_url = client
		.config
		.lock()
		.unwrap()
		.server_url
		.join("auth/v1/")
		.unwrap();
	let url = format!("{api_url}{}", R::PATH);

	let client = reqwest::Client::new();

	let res = client.post(url).json(req).send().await?;

	if !res.status().is_success() {
		let error = res.json::<auth::Error>().await?;

		return Err(Error::Api(error));
	}

	Ok(res.json::<R::Response>().await?)
}
