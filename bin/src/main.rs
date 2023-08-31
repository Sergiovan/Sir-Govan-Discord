use std::env;

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt::init();

	dotenv::dotenv().expect("Failed to load .env file");

	let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
	let beta = env::var("GOVAN_BETA").map_or(false, |res| res.to_lowercase() == "true");

	sirgovan::run(&token, beta).await;
}
