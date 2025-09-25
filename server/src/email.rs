use crate::config::Config;
use anyhow::Result;
use lettre::{
	Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
	message::{Mailbox, header::ContentType},
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Email {
	transport: AsyncSmtpTransport<Tokio1Executor>,
	noreply_sender: Address,
}

impl Email {
	pub fn init(config: &Config) -> Result<Self> {
		let transport =
			AsyncSmtpTransport::<Tokio1Executor>::from_url(config.email.noreply.as_str())?.build();

		let noreply_sender = urlencoding::decode(config.email.noreply.username())?.parse()?;

		Ok(Self {
			transport,
			noreply_sender,
		})
	}
	pub async fn send_email_verification(&self, recipient: &str, link_token: Uuid) -> Result<()> {
		let message = Message::builder()
			.from(Mailbox::new(
				Some("Salix".to_owned()),
				self.noreply_sender.clone(),
			))
			.to(recipient.parse()?)
			.subject("Salix account email confirmation")
			.header(ContentType::TEXT_PLAIN)
			.body(format!(
				"Hello!\n\nYour verification code is: {link_token}."
			))?;

		self.transport.send(message).await?;

		Ok(())
	}
	pub async fn send_reminder_about_account(&self, recipient: &str, username: &str) -> Result<()> {
		let message = Message::builder()
			.from(Mailbox::new(
				Some("Salix".to_owned()),
				self.noreply_sender.clone(),
			))
			.to(recipient.parse()?)
			.subject("Salix account reminder")
			.header(ContentType::TEXT_PLAIN)
			.body(format!(
				"Hello!\n\nSomeone attempted to create a new account with your email.
				If that was you, we remind you that you already have a Salix account with the username \"{username}\"."
			))?;

		self.transport.send(message).await?;

		Ok(())
	}
}
