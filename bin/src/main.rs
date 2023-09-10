use std::env;

use clap::Parser;

mod args;
mod tournaments;

#[tokio::main]
async fn main() {
	dotenv::dotenv().expect("Failed to load .env file");

	// console_subscriber::init();
	tracing_subscriber::fmt::init();

	let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
	let beta = env::var("GOVAN_BETA").map_or(false, |res| res.to_lowercase() == "true");

	let args = args::SirgovanArgs::parse();

	if args.commands.is_none() {
		sirgovan::run(&token, beta).await;
	} else {
		match args.commands.unwrap() {
			args::Commands::Tournament(args) => tournaments::run(&token, args).await,
		}
	}
}
