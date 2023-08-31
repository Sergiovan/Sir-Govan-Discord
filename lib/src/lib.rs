use std::convert::Infallible;

pub(crate) mod commands;
pub(crate) mod functionality;
pub(crate) mod handlers;
pub(crate) mod helpers;
pub(crate) mod prelude;

pub mod bot;
pub mod data;
pub mod event_handler;
pub mod util;

pub async fn run(token: &str, beta: bool) -> Option<Infallible> {
	use serenity::prelude::*;

	use bot::Bot;
	use data::BotData;
	use event_handler::BotEventHandler;
	use util::logger;
	use util::traits::ResultExt;

	let mut bot_data = BotData::new(beta);

	// TODO Separate these into their own check function
	bot_data
		.load_servers()
		.ok_or_log("Could not load servers file")?;
	bot_data
		.load_no_context()
		.ok_or_log("Could not load No Context Roles")?;
	bot_data
		.load_strings()
		.ok_or_log("Could not load Strings")?;

	let bot = std::sync::Arc::new(Bot::new(bot_data));

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
	bot.set_cache_and_http(client.cache_and_http.clone()).await;

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
		logger::error_fmt!("Client error: {:?}", why);
	}

	match tokio::time::timeout(tokio::time::Duration::from_secs(60), bot.shutdown()).await {
		Ok(_) => (),
		Err(e) => {
			logger::error_fmt!("Could not close down bot before a minute: {}", e);
		}
	}

	None
}
