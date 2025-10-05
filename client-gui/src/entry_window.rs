use crate::{EntryWindow, config::config, crate_version::version};
use async_compat::CompatExt;
use client::auth::{self, Error};
use slint::{ComponentHandle, ToSharedString};
use std::sync::Arc;
use tokio::sync::Mutex;

enum State {
	None,
	VerifyingEmail(auth::EmailVerifier),
	Finalizing(auth::CreateAccount),
}

pub fn entry_window() -> anyhow::Result<()> {
	let state = Arc::new(Mutex::new(State::None));

	let entry = EntryWindow::new()?;

	entry.set_build_info(version().into());

	let entry_weak = entry.as_weak();
	entry.on_login(move |email, password| {
		// let chat_window = ChatWindow::new().unwrap();
		// chat_window.show().unwrap();

		let entry = entry_weak.unwrap();
		slint::spawn_local(
			async move {
				match auth::login(&config(), email.to_string(), password.to_string()).await {
					Ok(auth_token) => {
						println!("LOGGED IN. auth token: {auth_token:?}");
						if let Err(e) = auth::store_auth_token(auth_token) {
							println!("error storing auth token: {e:?}");
						}
					}
					Err(e) => {
						println!("{e:?}");

						entry.set_login_error_message(e.to_shared_string());

						if matches!(e, auth::Error::Api(protocol::auth::v1::Error::Unauthorized)) {
							entry.set_login_email_error(true);
							entry.set_login_password_error(true);
						}
					}
				}

				entry.invoke_set_loading(false);
			}
			.compat(),
		)
		.unwrap();
	});

	let state_clone = Arc::clone(&state);
	let entry_weak = entry.as_weak();
	entry.on_register1(move |email| {
		let state = Arc::clone(&state_clone);
		let entry = entry_weak.unwrap();

		slint::spawn_local(
			async move {
				let res = auth::register(&config(), email.to_string()).await;

				entry.invoke_set_loading(false);

				let verifier = match res {
					Ok(x) => x,
					Err(e) => {
						println!("{e:?}");

						if matches!(e, auth::Error::InvalidEmail) {
							entry.set_register1_email_error(true);
						}

						entry.set_register1_error_message(e.to_shared_string());
						return;
					}
				};

				*state.lock().await = State::VerifyingEmail(verifier);

				// move to the next panel
				entry.set_current_panel(2);
			}
			.compat(),
		)
		.unwrap();
	});

	let state_weak = Arc::downgrade(&state);
	let entry_weak = entry.as_weak();
	entry.on_register2(move |code| {
		let state = state_weak.upgrade().unwrap();
		let entry = entry_weak.unwrap();

		slint::spawn_local(
			async move {
				let mut state = state.lock().await;

				let verifier = match &mut *state {
					State::VerifyingEmail(verifier) => verifier,
					_ => panic!("not supposed to be in this state"),
				};

				let code: u32 = code.parse().unwrap();

				let res = verifier.verify(&config(), code).await;

				entry.invoke_set_loading(false);

				let finalizer = match res {
					Ok(x) => x,
					Err(e) => {
						println!("{e:?}");
						entry.set_register2_error_message(e.to_shared_string());
						return;
					}
				};

				*state = State::Finalizing(finalizer);

				// move to the next panel
				entry.set_current_panel(3);
			}
			.compat(),
		)
		.unwrap();
	});

	let state_weak = Arc::downgrade(&state);
	let entry_weak = entry.as_weak();
	entry.on_register3(move |username, password| {
		let state = state_weak.upgrade().unwrap();
		let entry = entry_weak.unwrap();

		slint::spawn_local(
			async move {
				let state = state.lock().await;

				let finalizer = match &*state {
					State::Finalizing(finalizer) => finalizer,
					_ => panic!("not supposed to be in this state"),
				};

				let res = finalizer
					.finalize(&config(), username.to_string(), password.to_string())
					.await;

				entry.invoke_set_loading(false);

				let auth_token = match res {
					Err(e) => {
						println!("{e:?}");
						entry.set_register3_error_message(e.to_shared_string());

						if let Error::Api(protocol::auth::v1::Error::UsernameConflict) = e {
							entry.set_register3_username_error(true);
						}
						return;
					}
					Ok(id) => id,
				};

				println!("LOGGED IN. auth token: {auth_token:?}");
				if let Err(e) = auth::store_auth_token(auth_token) {
					println!("error storing auth token: {e:?}");
				}

				// move to the next panel
				entry.set_current_panel(4);
			}
			.compat(),
		)
		.unwrap();
	});

	entry.run()?;

	Ok(())
}
