use std::env;

#[tokio::main]
async fn main() {
	dotenv::dotenv().expect("Failed to load .env file");

	// console_subscriber::init();
	tracing_subscriber::fmt::init();

	let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
	let beta = env::var("GOVAN_BETA").map_or(false, |res| res.to_lowercase() == "true");

	sirgovan::run(&token, beta).await;
}
