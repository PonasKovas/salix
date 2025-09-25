use askama::Template;
use std::sync::OnceLock;

pub mod email_verification;

pub fn internal_error_page() -> &'static str {
	#[derive(Template)]
	#[template(path = "internal_error.html")]
	struct InternalErrorPage;

	static PAGE: OnceLock<String> = OnceLock::new();

	PAGE.get_or_init(|| InternalErrorPage.render().unwrap())
}
