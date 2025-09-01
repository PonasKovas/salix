use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
	server::main().await
}
