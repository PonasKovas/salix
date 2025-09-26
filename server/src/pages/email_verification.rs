use askama::Template;
use std::sync::OnceLock;

pub fn code_page(code: u32) -> String {
	#[derive(Template)]
	#[template(path = "email_verification_code.html")]
	struct CodePage<'a> {
		code: &'a str,
	}

	CodePage {
		code: &format!("{code:04}"),
	}
	.render()
	.unwrap()
}

pub fn code_not_found_page() -> &'static str {
	#[derive(Template)]
	#[template(path = "email_verification_code_invalid.html")]
	struct CodeNotFoundPage;

	static PAGE: OnceLock<String> = OnceLock::new();

	PAGE.get_or_init(|| CodeNotFoundPage.render().unwrap())
}
