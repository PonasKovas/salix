use crate::config::Config;
use anyhow::Result;
use lettre::{
	Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
	message::{Mailbox, header::ContentType},
};

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
	pub async fn send_email_confirmation(&self, recipient: &str, code: &str) -> Result<()> {
		let message = Message::builder()
			.from(Mailbox::new(
				Some("Salix".to_owned()),
				self.noreply_sender.clone(),
			))
			.to(recipient.parse()?)
			.subject("Salix account email confirmation")
			.header(ContentType::TEXT_PLAIN)
			.body(format!("Hello!\n\nYour verification code is: {code}."))?;

		self.transport.send(message).await?;

		Ok(())
	}
}
