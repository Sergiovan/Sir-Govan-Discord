use crate::bot::bot::Bot;
use crate::bot::data::BotData;
use crate::util::logging;

use serenity::prelude::*;
use serenity::model::prelude::*;
use colored::Colorize;

impl Bot {
  pub async fn on_message(&self, ctx: Context, msg: Message) {
    let data = ctx.data.read().await;
    let bot_data = data.get::<BotData>().unwrap().read().await;

    async fn log(ctx: &Context, msg: &Message) {
      let mine = msg.is_own(ctx);

      let author = if mine { "me".to_string() } else { msg.author.tag() };

      let channel = match msg.channel(&ctx).await {
        Ok(channel) => match channel {
          Channel::Guild(channel) => channel.name,
          Channel::Private(_) => if mine { "me".to_string() } else {ctx.cache.current_user().tag()}
          Channel::Category(channel) => channel.name,
          _ => "unknown-channel".to_string()
        },
        Err(_) => "unknown-channel".to_string()
      };

      let content = msg.content_safe(ctx) + " ";

      let attachments = if !msg.attachments.is_empty() {
        format!("[{} attachments] ", msg.attachments.len())
      } else {
        String::new()
      };

      let stickers = if !msg.sticker_items.is_empty() {
        format!("[{} stickers] ", msg.sticker_items.len())
      } else {
        String::new()
      };
      
      logging::info(&format!(
        "{} @ {}: {}{}{}", 
        author.cyan(), channel.cyan(), content, attachments.yellow(), stickers.yellow()
      ));
    }

    if msg.is_private() {
      log(&ctx, &msg).await;
    } else {
      let server = bot_data.servers.get(msg.guild_id.expect("Guild did not exist outside of DMs").as_u64());
      let Some(server) = server else { return };

      if server.channels.disallowed_listen.contains(msg.channel_id.as_u64()) {
        return;
      }
        
      log(&ctx, &msg).await;

      if msg.is_own(&ctx) {
        return;
      }

      // From here on we're for sure allowed to listen into messages

      // TODO Context Removal goes here

      // TODO Donk Solbs easter egg goes here

      if server.channels.allowed_commands.contains(msg.channel_id.as_u64()) {
        self.commander.read().await.parse(&ctx, &msg).await;
      }

    }
  }

}