use crate::{crate_version::version, logic::auth};
use async_compat::CompatExt;
use email_address::EmailAddress;
use slint::ToSharedString;

slint::include_modules!();

pub fn entry_window() -> anyhow::Result<()> {
	let entry = EntryWindow::new()?;

	entry.set_build_info(version().into());

	let entry_weak = entry.as_weak();
	entry.on_login(move |email, password| {
		let chat_window = ChatWindow::new().unwrap();
		chat_window.show().unwrap();

		let entry = entry_weak.unwrap();
		slint::spawn_local(
			async move {
				match auth::login(email.to_string(), password.to_string()).await {
					Ok(auth_token) => {
						println!("LOGGED IN. auth token: {auth_token}");
					}
					Err(e) => {
						entry.set_login_error(e.to_shared_string());

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

	let entry_weak = entry.as_weak();
	entry.on_register(move |email, username, password| {
		let entry = entry_weak.unwrap();

		if !EmailAddress::is_valid(&email) {
			entry.set_register_email_error(true);
			entry.set_register_error("invalid email".into());
			entry.invoke_set_loading(false);
			return;
		}

		slint::spawn_local(
			async move {
				let res = auth::register(
					email.to_string(),
					username.to_string(),
					password.to_string(),
				);

				match res.await {
					Ok(()) => {
						println!("account created");
					}
					Err(e) => {
						entry.set_register_error(e.to_shared_string());

						if let auth::LoginError::Api(api_err) = e {
							match api_err {
								protocol::auth::v1::Error::UsernameConflict => {
									entry.set_register_username_error(true);
								}
								protocol::auth::v1::Error::EmailConflict => {
									entry.set_register_email_error(true);
								}
								_ => {}
							}
						}
					}
				}

				entry.invoke_set_loading(false);
			}
			.compat(),
		)
		.unwrap();
	});

	entry.run()?;

	Ok(())
}
