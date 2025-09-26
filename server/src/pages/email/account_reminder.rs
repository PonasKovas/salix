use askama::Template;

pub fn account_reminder() -> String {
	#[derive(Template)]
	#[template(path = "email/account_reminder.html")]
	struct AccountReminder {}

	AccountReminder {}.render().unwrap()
}
