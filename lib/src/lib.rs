pub mod bot;
pub mod util;

pub async fn run(token: &str, beta: bool) {
	use serenity::prelude::*;

	use crate::bot::data::{BotData, ShardManagerContainer};
	use crate::bot::Bot;
	use crate::util::logger;

	tracing_subscriber::fmt::init();

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
		.event_handler(Bot::new(BotData::new(beta)))
		.await
		.expect("Err creating client");

	let shard_manager = client.shard_manager.clone();
	{
		let mut data = client.data.write().await;
		data.insert::<ShardManagerContainer>(shard_manager.clone());
	}

	tokio::spawn(async move {
		tokio::signal::ctrl_c()
			.await
			.expect("Could not register Ctrl+C handler");
		print!("\r");
		logger::debug("Bye!");
		shard_manager.lock().await.shutdown_all().await;
	});

	// Finally, start a single shard, and start listening to events.
	//
	// Shards will automatically attempt to reconnect, and will perform
	// exponential backoff until it reconnects.
	if let Err(why) = client.start().await {
		logger::error(&format!("Client error: {:?}", why));
	}
}
