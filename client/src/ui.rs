use crate::{
	crate_version::version,
	logic::auth::{self, LoginError},
};
use async_compat::CompatExt;
use email_address::EmailAddress;
use slint::ToSharedString;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

slint::include_modules!();

struct State {
	registration: RegistrationState,
}

enum RegistrationState {
	None,
	VerifyingEmail { email: String },
	Finalizing { registration_id: Uuid },
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
						entry.set_login_error_message(e.to_shared_string());

						if matches!(
							e,
							auth::LoginError::Api(protocol::auth::v1::Error::Unauthorized)
						) {
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

		if !EmailAddress::is_valid(&email) {
			entry.set_register1_email_error(true);
			entry.set_register1_error_message("invalid email".into());
			entry.invoke_set_loading(false);
			return;
		}

		slint::spawn_local(
			async move {
				let res = auth::start_verify_email(email.to_string()).await;

				entry.invoke_set_loading(false);

				if let Err(e) = res {
					println!("{e:?}");
					entry.set_register1_error_message(e.to_shared_string());
					return;
				}

				state.lock().unwrap().registration = RegistrationState::VerifyingEmail {
					email: email.to_string(),
				};

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
				let email = match &state.lock().unwrap().registration {
					RegistrationState::VerifyingEmail { email } => email.clone(),
					_ => panic!("not supposed to be in this state"),
				};

				let code: u32 = code.parse().unwrap();

				let res = auth::verify_email(email, code).await;

				entry.invoke_set_loading(false);

				let registration_id = match res {
					Err(e) => {
						println!("{e:?}");
						entry.set_register2_error_message(e.to_shared_string());
						return;
					}
					Ok(id) => id,
				};

				state.lock().unwrap().registration =
					RegistrationState::Finalizing { registration_id };

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
				let registration_id = match &state.lock().unwrap().registration {
					RegistrationState::Finalizing { registration_id } => *registration_id,
					_ => panic!("not supposed to be in this state"),
				};

				let res =
					auth::finalize(registration_id, username.to_string(), password.to_string())
						.await;

				entry.invoke_set_loading(false);

				let auth_token = match res {
					Err(e) => {
						println!("{e:?}");
						entry.set_register3_error_message(e.to_shared_string());

						if let LoginError::Api(protocol::auth::v1::Error::UsernameConflict) = e {
							entry.set_register3_username_error(true);
						}
						return;
					}
					Ok(id) => id,
				};

				println!("LOGGED IN. auth token: {auth_token}");

				entry.set_current_panel(4);
			}
			.compat(),
		)
		.unwrap();
	});

	entry.run()?;

	Ok(())
}
