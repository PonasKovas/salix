use server::pages;
use std::{fs, path::Path};

fn main() {
	println!("🎨 Generating page template previews...");

	let output_dir = Path::new("page-previews");
	fs::create_dir_all(output_dir).unwrap();

	save(
		&output_dir.join("internal_error.html"),
		pages::internal_error_page(),
	);
	save(
		&output_dir.join("email_verification_code_invalid.html"),
		pages::email_verification::code_not_found_page(),
	);
	save(
		&output_dir.join("email_verification_code.html"),
		&pages::email_verification::code_page(0123),
	);

	// emails
	//////////

	let output_dir = output_dir.join("emails");
	fs::create_dir_all(&output_dir).unwrap();
	save(
		&output_dir.join("verification_code.html"),
		&pages::email::verification_code("https://example.com/"),
	);
	save(
		&output_dir.join("account_reminder.html"),
		&pages::email::account_reminder(),
	);
}

fn save(output_path: &Path, contents: &str) {
	fs::write(&output_path, contents).unwrap();

	println!("✅ Preview saved to '{}'", output_path.display());
}
