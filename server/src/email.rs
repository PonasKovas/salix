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
	pub async fn send_noreply_email(
		&self,
		recipient: &str,
		subject: &str,
		html: &str,
	) -> Result<()> {
		// emails sometimes need to have CSS inlined to render correctly
		let inliner = css_inline::CSSInliner::options()
			.load_remote_stylesheets(false)
			.build();
		let inlined_html = inliner.inline(html)?;

		let message = Message::builder()
			.from(Mailbox::new(
				Some("Salix".to_owned()),
				self.noreply_sender.clone(),
			))
			.to(recipient.parse()?)
			.subject(subject)
			.header(ContentType::TEXT_HTML)
			.body(inlined_html)?;

		self.transport.send(message).await?;

		Ok(())
	}
}
