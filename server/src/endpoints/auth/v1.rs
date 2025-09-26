use crate::{
	ServerState,
	database::{
		email_verifications::{EmailAlreadyAdded, VerifyEmailError},
		user::UsernameConflict,
	},
	pages::{
		self,
		email_verification::{code_not_found_page, code_page},
		internal_error_page,
	},
};
use anyhow::Context;
use argon2::{
	Algorithm, Argon2, Params, PasswordVerifier, Version,
	password_hash::{SaltString, rand_core::OsRng},
};
use argon2::{PasswordHash, PasswordHasher};
use axum::{
	Json, Router,
	extract::State,
	response::{Html, IntoResponse},
	routing::{get, post},
};
use axum::{extract::Path, http::StatusCode};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::Utc;
use email_address::EmailAddress;
use protocol::auth::{
	Request,
	v1::{self, *},
};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

const AUTH_TOKEN_LIFETIME: u32 = 15 * 24; // in hours
const EMAIL_VERIFICATION_LIFETIME: u32 = 60; // in minutes
const REGISTRATION_LIFETIME: u32 = 60; // in minutes
const EMAIL_ACCOUNT_REMIND_TIMEOUT: i64 = 7 * 24; // in hours

#[derive(Error, Debug)]
pub enum Error {
	#[error("database: {0}")]
	Database(#[from] sqlx::Error),
	#[error(transparent)]
	Internal(#[from] anyhow::Error),
	#[error(transparent)]
	Api(#[from] v1::Error),
}

impl IntoResponse for Error {
	fn into_response(self) -> axum::response::Response {
		error!("{self:?}");
		match self {
			Error::Database(_) | Error::Internal(_) | Error::Api(v1::Error::Internal) => {
				(StatusCode::INTERNAL_SERVER_ERROR, Json(v1::Error::Internal))
			}
			Error::Api(e) => (StatusCode::BAD_REQUEST, Json(e)),
		}
		.into_response()
	}
}

pub fn routes() -> Router<ServerState> {
	Router::new()
		.route(LoginRequest::PATH, post(login))
		.route(StartEmailVerifyRequest::PATH, post(start_email_verify))
		.route(VerifyEmailRequest::PATH, post(verify_email))
		.route(FinalizeNewAccountRequest::PATH, post(finalize_new_account))
		.route("/verify/{token}", get(display_verification_code))
}

async fn start_email_verify(
	State(mut state): State<ServerState>,
	Json(request): Json<StartEmailVerifyRequest>,
) -> Result<Json<()>, Error> {
	if !EmailAddress::is_valid(&request.email) {
		return Err(v1::Error::InvalidRequest.into());
	}

	match state.db.user_by_email(&request.email).await? {
		Some(user) => {
			// some user is already registered with this email.
			// Send a reminder email that an account already exists

			let since_last_reminder = Utc::now() - user.last_account_reminder_sent;

			if since_last_reminder.num_hours() >= EMAIL_ACCOUNT_REMIND_TIMEOUT {
				state
					.email
					.send_noreply_email(
						&request.email,
						"Salix Account Notice",
						&pages::email::account_reminder(),
					)
					.await?;
			}
		}
		None => {
			let link_token = Uuid::now_v7();
			let link_token_hash = hash_link_token(link_token);

			let code = rand::rng().random_range(0..10000); // 4 digits

			if let Err(EmailAlreadyAdded) = state
				.db
				.insert_email_verification(
					&request.email,
					&link_token_hash,
					code,
					EMAIL_VERIFICATION_LIFETIME,
				)
				.await?
			{
				// email must be already sent. no need to send it again, and dont let the user know that
				// the email was already added
				return Ok(Json(()));
			}

			let link = state
				.config
				.public_base_url
				.join("auth/v1/verify/")
				.unwrap()
				.join(&link_token.to_string())
				.unwrap();

			state
				.email
				.send_noreply_email(
					&request.email,
					"Salix Verification Code",
					&pages::email::verification_code(link.as_str()),
				)
				.await?;
		}
	}

	Ok(Json(()))
}

async fn verify_email(
	State(mut state): State<ServerState>,
	Json(request): Json<VerifyEmailRequest>,
) -> Result<Json<VerifyEmailResponse>, Error> {
	if let Err(e) = state.db.verify_email(&request.email, request.code).await? {
		match e {
			VerifyEmailError::IncorrectCode => return Err(v1::Error::IncorrectCode.into()),
			VerifyEmailError::TooManyAttempts => return Err(v1::Error::TooManyAttempts.into()),
			VerifyEmailError::Invalid => return Err(v1::Error::InvalidRequest.into()),
		}
	}

	// code verified, create a new registration id
	let registration_id = Uuid::now_v7();

	// client will use this registration id to finalize the registration with a username and password
	state
		.db
		.insert_registration(registration_id, &request.email, REGISTRATION_LIFETIME)
		.await?;

	Ok(Json(VerifyEmailResponse { registration_id }))
}

async fn finalize_new_account(
	State(mut state): State<ServerState>,
	Json(request): Json<FinalizeNewAccountRequest>,
) -> Result<Json<LoginSuccess>, Error> {
	let mut transaction = state.db.transaction().await?;

	let registration = match transaction
		.get_registration(request.registration_id)
		.await?
	{
		Some(x) => x,
		None => return Err(v1::Error::InvalidRequest.into()),
	};

	// hash the password
	let salt = SaltString::generate(&mut OsRng);
	let password_hash = argon2()
		.hash_password(request.password.as_bytes(), &salt)
		.with_context(|| format!("{request:?}"))?
		.to_string();

	let user_id = transaction
		.insert_user(&request.username, &registration.email, &password_hash)
		.await?
		.map_err(|UsernameConflict| v1::Error::UsernameConflict)?;

	transaction
		.remove_registration(request.registration_id)
		.await?;

	transaction.commit().await?;

	let auth_token = new_auth_session(&mut state, user_id).await?;

	Ok(Json(LoginSuccess { auth_token }))
}

async fn login(
	State(mut state): State<ServerState>,
	Json(request): Json<LoginRequest>,
) -> Result<Json<LoginSuccess>, Error> {
	let user = state
		.db
		.user_by_email(&request.email)
		.await?
		.ok_or(v1::Error::Unauthorized)?;

	let hash = PasswordHash::new(&user.password).with_context(|| format!("{user:?}"))?;

	argon2()
		.verify_password(request.password.as_bytes(), &hash)
		.map_err(|_| v1::Error::Unauthorized)?;

	let token = new_auth_session(&mut state, user.id).await?;

	Ok(Json(LoginSuccess { auth_token: token }))
}

async fn display_verification_code(
	State(mut state): State<ServerState>,
	Path(link_token): Path<Uuid>,
) -> Html<Cow<'static, str>> {
	let hash = hash_link_token(link_token);

	let code = match state.db.get_email_verification_code(&hash).await {
		Ok(Some(x)) => x,
		Ok(None) => {
			return Html(code_not_found_page().into());
		}
		Err(e) => {
			error!("error getting email verification code (link token {link_token}): {e:?}");
			return Html(internal_error_page().into());
		}
	};

	Html(code_page(code).into())
}

fn hash_link_token(token: Uuid) -> String {
	BASE64_STANDARD.encode(Sha256::digest(token.as_bytes()))
}

async fn new_auth_session(state: &mut ServerState, user_id: Uuid) -> sqlx::Result<Uuid> {
	state
		.db
		.insert_active_session(user_id, AUTH_TOKEN_LIFETIME)
		.await
}

fn argon2() -> Argon2<'static> {
	Argon2::new(Algorithm::default(), Version::default(), Params::default())
}
