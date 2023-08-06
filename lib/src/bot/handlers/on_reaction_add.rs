use std::convert::Infallible;

use crate::bot::data::EmojiType;
use crate::bot::Bot;
use crate::util::{logger, CacheGuild, ResultErrorHandler};

use serenity::model::prelude::*;
use serenity::prelude::*;

impl Bot {
	pub async fn on_reaction_add(
		&self,
		ctx: Context,
		add_reaction: Reaction,
	) -> Option<Infallible> {
		let reactor = add_reaction.user(&ctx).await.unwrap_or_log(&format!(
			"Could not determine reactor for reaction {:?}",
			add_reaction
		))?;

		if reactor.id == ctx.cache.current_user_id() {
			return None;
		}

		let msg = add_reaction
			.message(&ctx.http)
			.await
			.unwrap_or_log(&format!(
				"Message {} that was reacted to with {} could not be fetched",
				add_reaction.message_id, add_reaction.emoji
			))?;

		if !msg.guild_cached(&ctx).await {
			return None;
		}

		if msg.is_own(&ctx) {
			return None;
		}

		// So, msg.is_private() won't work because messages fetched through the REST API don't come with
		// a guild_id, which means msg.is_private() will always be true
		let this_channel = msg.channel(&ctx).await.unwrap_or_log(&format!(
			"Message {}'s channel {} could not be fetched",
			msg.id, msg.channel_id
		))?;

		let this_channel = this_channel.guild()?;

		enum Action {
			Pin {
				destination_id: u64,
				required: u32,
				emoji_override: Option<EmojiType>,
			},
			None,
		}

		let action = {
			let bot_data = self.data.read().await;

			let server = bot_data.servers.get(this_channel.guild_id.as_u64())?;

			if server
				.channels
				.disallowed_listen
				.contains(msg.channel_id.as_u64())
			{
				return None;
			}

			let emoji: EmojiType = (&add_reaction.emoji).into();
			let required = server.pin_amount;

			// Decide what to do here
			if server.is_fame_emoji(&emoji) {
				let hall = server.hall_of_fame.as_ref().unwrap();
				let channel_id = hall.channel;

				Action::Pin {
					destination_id: channel_id,
					required,
					emoji_override: {
						if let EmojiType::Unicode(emoji) = hall.get_emoji() {
							if emoji.contains(crate::bot::data::emoji::PIN) {
								Some(crate::bot::data::emoji::REDDIT_GOLD)
							} else {
								None
							}
						} else {
							None
						}
					},
				}
			} else if server.is_typo_emoji(&emoji) {
				let hall = server.hall_of_typo.as_ref().unwrap();
				let channel_id = hall.channel;

				Action::Pin {
					destination_id: channel_id,
					required,
					emoji_override: None,
				}
			} else if server.is_vague_emoji(&emoji) {
				let hall = server.hall_of_vague.as_ref().unwrap();
				let channel_id = hall.channel;

				Action::Pin {
					destination_id: channel_id,
					required,
					emoji_override: None,
				}
			} else if let Some(hall) = server.hall_of_all.as_ref() {
				let channel_id = hall.channel;

				Action::Pin {
					destination_id: channel_id,
					required,
					emoji_override: None,
				}
			} else {
				Action::None
			}
		};

		match action {
			Action::None => None,
			Action::Pin {
				destination_id,
				required,
				emoji_override,
			} => {
				let channel = match ctx.cache.guild_channel(destination_id) {
					Some(channel) => channel,
					None => match ctx.http.get_channel(destination_id).await {
						Ok(Channel::Guild(channel)) => channel,
						Ok(c) => {
							logger::error(&format!("Channel {} for hall emoji {} is misconfigured, not a guild channel: {}", destination_id, add_reaction.emoji, c));
							return None;
						}
						Err(e) => {
							logger::error(&format!(
								"Error when fetching channel {}: {}",
								destination_id, e
							));
							return None;
						}
					},
				};

				if !channel.guild_cached(&ctx).await {
					return None;
				}

				self.maybe_pin(ctx, msg, add_reaction, channel, required, emoji_override)
					.await;
				None
			}
		}
	}
}
