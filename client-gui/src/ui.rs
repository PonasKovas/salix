use crate::crate_version::version;
use async_compat::CompatExt;
use client::auth::{self, Error};
use slint::{Global, ToSharedString};
use std::sync::Arc;
use tokio::sync::Mutex;

slint::include_modules!();

struct State {
	registration: RegistrationState,
}

enum RegistrationState {
	None,
	VerifyingEmail(auth::EmailVerifier),
	Finalizing(auth::CreateAccount),
}

pub fn entry_window() -> anyhow::Result<()> {
	let state = Arc::new(Mutex::new(State {
		registration: RegistrationState::None,
	}));

	let entry = EntryWindow::new()?;

	entry.set_build_info(version().into());

	let entry_weak = entry.as_weak();
	entry.on_login(move |email, password| {
		// let chat_window = ChatWindow::new().unwrap();
		// chat_window.show().unwrap();

		let entry = entry_weak.unwrap();
		slint::spawn_local(
			async move {
				match auth::login(email.to_string(), password.to_string()).await {
					Ok(auth_token) => {
						println!("LOGGED IN. auth token: {auth_token}");
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

	let state_weak = Arc::downgrade(&state);
	let entry_weak = entry.as_weak();
	entry.on_register1(move |email| {
		let state = state_weak.upgrade().unwrap();
		let entry = entry_weak.unwrap();

		slint::spawn_local(
			async move {
				let res = auth::register(email.to_string()).await;

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

				state.lock().await.registration = RegistrationState::VerifyingEmail(verifier);

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

				let verifier = match &mut state.registration {
					RegistrationState::VerifyingEmail(verifier) => verifier,
					_ => panic!("not supposed to be in this state"),
				};

				let code: u32 = code.parse().unwrap();

				let res = verifier.verify(code).await;

				entry.invoke_set_loading(false);

				let finalizer = match res {
					Ok(x) => x,
					Err(e) => {
						println!("{e:?}");
						entry.set_register2_error_message(e.to_shared_string());
						return;
					}
				};

				state.registration = RegistrationState::Finalizing(finalizer);

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

				let finalizer = match &state.registration {
					RegistrationState::Finalizing(finalizer) => finalizer,
					_ => panic!("not supposed to be in this state"),
				};

				let res = finalizer
					.finalize(username.to_string(), password.to_string())
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
