use crate::bot::bot::Bot;
use crate::util::logging;
use crate::bot::data::{BotData, ShardManagerContainer};

use serenity::prelude::*;
use serenity::model::prelude::*;

impl Bot {
  pub async fn on_ready(&self, ctx: Context, ready: Ready) {
    logging::debug("Getting ready...");
    self.commander.write().await.register_all();

    let data = ctx.data.read().await;

    if let Some(bot_data) = data.get::<BotData>() {
      use crate::bot::data::config;
      let data = match config::read_servers() {
        Ok(data) => data,
        Err(config::Error::IO(err)) => {
          logging::error(&format!("Could not open the settings file: {}", err));
          data.get::<ShardManagerContainer>().unwrap().lock().await.shutdown_all().await;
          return;
        },
        Err(config::Error::Toml(err)) => {
          logging::error(&format!("Could not parse the settings file: {}", err));
          data.get::<ShardManagerContainer>().unwrap().lock().await.shutdown_all().await;
          return;
        }
      };

      let is_beta = bot_data.read().await.beta;

      bot_data.write().await.servers.extend(
        data.servers.into_iter().filter(|server| server.beta == is_beta).map(|server| (server.id, server.into()))
      );

      logging::info(
        &format!("Am ready :). I am {}. I am in {} mode", 
          ready.user.tag(), 
          if bot_data.read().await.beta { "beta" } else { "normal" })
      );
    } else {
      logging::error("Error getting the server list!");
      data.get::<ShardManagerContainer>().unwrap().lock().await.shutdown_all().await;
      return;
    }
  }
}