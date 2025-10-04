use email_address::EmailAddress;
use protocol::auth::Request;
use protocol::auth::v1 as auth;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum Error {
	#[error(transparent)]
	Reqwest(#[from] reqwest::Error),
	#[error(transparent)]
	Api(#[from] auth::Error),
	/// for verifying an email
	#[error("too many attempts")]
	TooManyAttempts,
	#[error("invalid email address")]
	InvalidEmail,
}

// todo replace this with a config struct
const BASE_API_URL: &'static str = "http://localhost:3000/auth/v1";

async fn make_request<R: Request>(req: &R) -> Result<R::Response, Error> {
	let url = format!("{BASE_API_URL}{}", R::PATH);

	let client = reqwest::Client::new();

	let res = client.post(url).json(req).send().await?;

	if !res.status().is_success() {
		let error = res.json::<auth::Error>().await?;

		return Err(Error::Api(error));
	}

	Ok(res.json::<R::Response>().await?)
}

pub async fn login(email: String, password: String) -> Result<Uuid, Error> {
	make_request(&auth::LoginRequest { email, password })
		.await
		.map(|success| success.auth_token)
}

pub async fn register(email: String) -> Result<EmailVerifier, Error> {
	if !EmailAddress::is_valid(&email) {
		return Err(Error::InvalidEmail);
	}

	let request = auth::StartEmailVerifyRequest { email };
	make_request(&request).await?;

	Ok(EmailVerifier {
		email: Some(request.email),
		attempts: 0,
	})
}

#[derive(Debug, Clone)]
pub struct EmailVerifier {
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

		let registration_id = match make_request(&request)
			.await
			.map(|response| response.registration_id)
		{
			Ok(x) => x,
			Err(e) => {
				self.email = Some(request.email);
				return Err(e);
			}
		};

		Ok(CreateAccount { registration_id })
	}
}

#[derive(Debug, Clone)]
pub struct CreateAccount {
	registration_id: Uuid,
}

impl CreateAccount {
	pub async fn finalize(&self, username: String, password: String) -> Result<AuthToken, Error> {
		let response = make_request(&auth::FinalizeNewAccountRequest {
			registration_id: self.registration_id,
			username,
			password,
		})
		.await?;

		Ok(AuthToken(response.auth_token))
	}
}

#[derive(Debug, Clone)]
pub struct AuthToken(pub(crate) Uuid);
