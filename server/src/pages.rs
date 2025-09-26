use askama::Template;
use std::sync::OnceLock;

pub mod email;
pub mod email_verification;

/// used by the templates themselves
pub mod palette {
	pub const BACKGROUND1: &'static str = "#1A1D1A";
	pub const BACKGROUND2: &'static str = "#242824";
	pub const TEXT1: &'static str = "#E4E6EB";
	pub const TEXT2: &'static str = "#A8B3A8";
	pub const ERROR: &'static str = "#E54B4B";
	pub const ACCENT: &'static str = "#73AB84";
}

pub const PREHEADER_WHITESPACE: &'static [&'static str] =
	&["&#847; &zwnj; &nbsp; &#8199; &shy; "; 200];

pub fn internal_error_page() -> &'static str {
	#[derive(Template)]
	#[template(path = "internal_error.html")]
	struct InternalErrorPage;

	static PAGE: OnceLock<String> = OnceLock::new();

	PAGE.get_or_init(|| InternalErrorPage.render().unwrap())
}
