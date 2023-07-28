mod bot;
mod util;

use std::env;
use serenity::prelude::*;

use bot::Bot;
use util::logging;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");

    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::DIRECT_MESSAGE_REACTIONS;

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client =
        Client::builder(&token, intents).event_handler(Bot).await.expect("Err creating client");

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Could not register Ctrl+C handler");
        logging::debug("Bye!");
        shard_manager.lock().await.shutdown_all().await;
    });

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        logging::error(&format!("Client error: {:?}", why));
    }
}