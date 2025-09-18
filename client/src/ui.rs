use crate::{crate_version::version, logic::auth};
use async_compat::CompatExt;
use slint::ToSharedString;

slint::include_modules!();

pub fn entry_window() -> anyhow::Result<()> {
	let entry = EntryWindow::new()?;

	entry.set_build_info(version().into());

	let entry_weak = entry.as_weak();
	entry.on_login(move |username, password| {
		let entry = entry_weak.unwrap();
		slint::spawn_local(
			async move {
				match auth::login(username.to_string(), password.to_string()).await {
					Ok(auth_token) => {
						println!("LOGGED IN. auth token: {auth_token}");
					}
					Err(e) => {
						entry.set_login_error(e.to_shared_string());

						if matches!(
							e,
							auth::LoginError::Api(protocol::auth::v1::Error::Unauthorized)
						) {
							entry.set_login_username_error(true);
							entry.set_login_password_error(true);
						}
						entry.invoke_set_loading(false);
					}
				}
			}
			.compat(),
		)
		.unwrap();
	});

	entry.run()?;

	Ok(())
}
