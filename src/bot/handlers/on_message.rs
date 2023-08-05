use std::convert::Infallible;

use crate::bot::data::BotData;
use crate::bot::Bot;
use crate::util::{logger, CacheGuild};

use colored::Colorize;
use serenity::model::prelude::*;
use serenity::prelude::*;

impl Bot {
	pub async fn on_message(&self, ctx: Context, msg: Message) -> Option<Infallible> {
		if !msg.guild_cached(&ctx).await {
			return None;
		}

		let data = ctx.data.read().await;
		let bot_data = data.get::<BotData>().unwrap().read().await;

		async fn log(ctx: &Context, msg: &Message) {
			let mine = msg.is_own(ctx);

			let author = if mine {
				"me".to_string()
			} else {
				msg.author.tag()
			};

			let channel = match msg.channel(&ctx).await {
				Ok(channel) => match channel {
					Channel::Guild(channel) => channel.name,
					Channel::Private(channel) => {
						if mine {
							channel.recipient.name
						} else {
							"me".to_string()
						}
					}
					Channel::Category(channel) => channel.name,
					_ => "unknown-channel".to_string(),
				},
				Err(_) => "unknown-channel".to_string(),
			};

			let content = msg.content_safe(ctx) + " ";

			let attachments = if !msg.attachments.is_empty() {
				format!("[{} attachments] ", msg.attachments.len())
			} else {
				String::new()
			};

			let embeds = if !msg.embeds.is_empty() {
				format!("[{} embeds] ", msg.embeds.len())
			} else {
				String::new()
			};

			let stickers = if !msg.sticker_items.is_empty() {
				format!("[{} stickers] ", msg.sticker_items.len())
			} else {
				String::new()
			};

			logger::info(&format!(
				"{} @ {}: {}{}{}{}",
				author.cyan(),
				channel.cyan(),
				content,
				attachments.yellow(),
				embeds.yellow(),
				stickers.yellow()
			));
		}

		if msg.is_private() {
			log(&ctx, &msg).await;
		} else {
			let server = bot_data.servers.get(
				msg.guild_id
					.expect("Guild did not exist outside of DMs")
					.as_u64(),
			)?;

			if server
				.channels
				.disallowed_listen
				.contains(msg.channel_id.as_u64())
			{
				return None;
			}

			log(&ctx, &msg).await;

			if msg.is_own(&ctx) {
				return None;
			}

			// From here on we're for sure allowed to listen into messages

			// TODO Context Removal goes here

			// TODO Donk Solbs easter egg goes here

			if server
				.channels
				.allowed_commands
				.contains(msg.channel_id.as_u64())
			{
				self.commander.read().await.parse(&ctx, &msg).await;
			}
		}
		None
	}
}
