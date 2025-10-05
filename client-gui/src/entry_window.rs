use crate::{EntryWindow, chat_window, crate_version::version, error_window::show_error_window};
use anyhow::Result;
use anyhow::anyhow;
use async_compat::CompatExt;
use client::{
	Client,
	auth::{self, AuthToken, Error},
};
use slint::{ComponentHandle, SharedString, ToSharedString};
use std::sync::Arc;
use tokio::sync::Mutex;

enum State {
	None,
	VerifyingEmail(auth::EmailVerifier),
	Finalizing(auth::CreateAccount),
}

pub fn entry_window(client: Client) -> anyhow::Result<()> {
	let state = Arc::new(Mutex::new(State::None));

	let entry = EntryWindow::new()?;

	entry.set_build_info(version().into());

	let entry_weak = entry.as_weak();
	let client_clone = client.clone();
	entry.on_login(move |email, password| {
		let entry = entry_weak.unwrap();
		let client = client_clone.clone();

		slint::spawn_local(on_login(client, entry, email, password).compat()).unwrap();
	});

	let state_clone = Arc::clone(&state);
	let entry_weak = entry.as_weak();
	let client_clone = client.clone();
	entry.on_register1(move |email| {
		let state = Arc::clone(&state_clone);
		let entry = entry_weak.unwrap();
		let client = client_clone.clone();

		slint::spawn_local(on_register1(client, entry, state, email).compat()).unwrap();
	});

	let state_clone = Arc::clone(&state);
	let entry_weak = entry.as_weak();
	let client_clone = client.clone();
	entry.on_register2(move |code| {
		let state = Arc::clone(&state_clone);
		let entry = entry_weak.unwrap();
		let client = client_clone.clone();

		slint::spawn_local(on_register2(client, entry, state, code).compat()).unwrap();
	});

	let state_clone = Arc::clone(&state);
	let entry_weak = entry.as_weak();
	let client_clone = client.clone();
	entry.on_register3(move |username, password| {
		let state = Arc::clone(&state_clone);
		let entry = entry_weak.unwrap();
		let client = client_clone.clone();

		slint::spawn_local(on_register3(client, entry, state, username, password).compat())
			.unwrap();
	});

	entry.show()?;

	Ok(())
}

async fn on_login(
	client: Client,
	entry: EntryWindow,
	email: SharedString,
	password: SharedString,
) -> Result<()> {
	let res = client
		.auth
		.login(email.to_string(), password.to_string())
		.await;

	entry.invoke_set_loading(false);

	match res {
		Ok(auth_token) => {
			finish(client, entry, auth_token);
		}
		Err(e) => {
			entry.set_login_error_message(e.to_shared_string());

			if matches!(e, auth::Error::Api(protocol::auth::v1::Error::Unauthorized)) {
				entry.set_login_email_error(true);
				entry.set_login_password_error(true);
			}

			println!("{:?}", anyhow!(e));
		}
	}

	Ok(())
}

async fn on_register1(
	client: Client,
	entry: EntryWindow,
	state: Arc<Mutex<State>>,
	email: SharedString,
) -> Result<()> {
	let res = client.auth.register(email.to_string()).await;

	entry.invoke_set_loading(false);

	let verifier = match res {
		Ok(x) => x,
		Err(e) => {
			entry.set_register1_error_message(e.to_shared_string());

			if matches!(e, auth::Error::InvalidEmail) {
				entry.set_register1_email_error(true);
			}

			println!("{:?}", anyhow!(e));

			return Ok(());
		}
	};

	*state.lock().await = State::VerifyingEmail(verifier);

	// move to the next panel
	entry.set_current_panel(2);

	Ok(())
}

async fn on_register2(
	_client: Client,
	entry: EntryWindow,
	state: Arc<Mutex<State>>,
	code: SharedString,
) -> Result<()> {
	let mut state = state.lock().await;

	let verifier = match &mut *state {
		State::VerifyingEmail(verifier) => verifier,
		_ => panic!("not supposed to be in this state"),
	};

	// in slint the input type is number and it enforces max 4 digits so this will never fail
	let code: u32 = code.parse().unwrap();

	let res = verifier.verify(code).await;

	entry.invoke_set_loading(false);

	let finalizer = match res {
		Ok(x) => x,
		Err(e) => {
			entry.set_register2_error_message(e.to_shared_string());

			println!("{:?}", anyhow!(e));

			return Ok(());
		}
	};

	*state = State::Finalizing(finalizer);

	// move to the next panel
	entry.set_current_panel(3);

	Ok(())
}

async fn on_register3(
	client: Client,
	entry: EntryWindow,
	state: Arc<Mutex<State>>,
	username: SharedString,
	password: SharedString,
) -> Result<()> {
	let state = state.lock().await;

	let finalizer = match &*state {
		State::Finalizing(finalizer) => finalizer,
		_ => panic!("not supposed to be in this state"),
	};

	let res = finalizer
		.finalize(username.to_string(), password.to_string())
		.await;

	entry.invoke_set_loading(false);

	let auth_token = match res {
		Err(e) => {
			entry.set_register3_error_message(e.to_shared_string());

			if let Error::Api(protocol::auth::v1::Error::UsernameConflict) = e {
				entry.set_register3_username_error(true);
			}

			println!("{:?}", anyhow!(e));

			return Ok(());
		}
		Ok(id) => id,
	};

	finish(client, entry, auth_token);

	Ok(())
}

fn finish(client: Client, entry: EntryWindow, token: AuthToken) {
	// this closure is called either when the user successfully logins
	// or creates a new account
	// either way receives an auth_token

	println!("LOGGED IN. auth token: {token:?}");

	if let Err(e) = client::auth_token_store::store_auth_token(&token) {
		show_error_window(slint::format!("error storing auth token: {e}"), false).unwrap();
	}

	client.set_auth_token(token);

	chat_window::chat_window(client).unwrap();
	entry.hide().unwrap();
}
