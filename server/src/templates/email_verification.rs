use askama::Template;
use std::sync::OnceLock;

#[derive(Template)]
#[template(path = "email_verification_code.html")]
pub struct CodePage {
	pub code: u32,
}

pub fn code_not_found_page() -> &'static str {
	#[derive(Template)]
	#[template(path = "email_verification_code_invalid.html")]
	struct CodeNotFoundPage;

	static PAGE: OnceLock<String> = OnceLock::new();

	PAGE.get_or_init(|| CodeNotFoundPage.render().unwrap())
}
