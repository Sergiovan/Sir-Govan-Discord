pub mod bot;
pub mod event_handler;
pub mod util;

pub async fn run(token: &str, beta: bool) {
	use serenity::prelude::*;

	use crate::bot::data::BotData;
	use crate::bot::Bot;
	use crate::event_handler::BotEventHandler;
	use crate::util::logger;

	tracing_subscriber::fmt::init();

	let bot = std::sync::Arc::new(Bot::new(BotData::new(beta)));

	// Set gateway intents, which decides what events the bot will be notified about
	let intents = GatewayIntents::GUILDS
		| GatewayIntents::GUILD_MESSAGES
		| GatewayIntents::DIRECT_MESSAGES
		| GatewayIntents::MESSAGE_CONTENT
		| GatewayIntents::GUILD_MESSAGE_REACTIONS
		| GatewayIntents::DIRECT_MESSAGE_REACTIONS;

	// Create a new instance of the Client, logging in as a bot. This will
	// automatically prepend your bot token with "Bot ", which is a requirement
	// by Discord for bot users.
	let mut client = Client::builder(token, intents)
		.event_handler(BotEventHandler::new(bot.clone()))
		.await
		.expect("Err creating client");

	let shard_manager = client.shard_manager.clone();
	bot.set_shard_manager(shard_manager).await;

	{
		let bot = bot.clone();
		tokio::spawn(async move {
			tokio::signal::ctrl_c()
				.await
				.expect("Could not register Ctrl+C handler");
			print!("\r");
			bot.shutdown().await;
		});
	}

	if let Err(why) = client.start().await {
		logger::error(&format!("Client error: {:?}", why));
	}

	match tokio::time::timeout(tokio::time::Duration::from_secs(60), bot.shutdown()).await {
		Ok(_) => (),
		Err(e) => {
			logger::error(&format!("Could not close down bot before a minute: {}", e));
		}
	}
}
