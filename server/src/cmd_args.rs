use clap::Parser;
use std::path::PathBuf;

/// Salix server implementation
#[derive(Parser, Debug)]
#[command(version)]
pub struct Args {
	/// Path to the salix.toml configuration file
	#[arg(short, long, default_value = "salix.toml")]
	pub config: PathBuf,
	/// Dont migrate database
	#[arg(short, long)]
	pub no_migrate: bool,
	/// Populate the database with dummy dev data
	#[arg(short, long)]
	pub populate: bool,
}
