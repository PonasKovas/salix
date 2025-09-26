use askama::Template;

pub fn verification_code(verification_link: &str) -> String {
	#[derive(Template)]
	#[template(path = "email/verification_code.html")]
	struct CodePage<'a> {
		verification_link: &'a str,
	}

	CodePage { verification_link }.render().unwrap()
}
