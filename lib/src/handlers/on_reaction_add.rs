use crate::prelude::*;

use crate::bot::Bot;
use crate::data::EmojiType;
use crate::util::error::GovanResult;

use colored::Colorize;
use serenity::model::prelude::*;
use serenity::prelude::*;

impl Bot {
	pub async fn on_reaction_add(&self, ctx: &Context, add_reaction: &Reaction) -> GovanResult {
		let this_channel = add_reaction.channel(&ctx).await?;

		let this_channel = this_channel.guild().ok_or_else(govanerror::debug_lazy!(
			log = "Not in a guild channel" //, user = "You can only use this inside a guild!"
		))?;

		let bot_data = self.data().await;

		let server = bot_data
			.servers
			.get(this_channel.guild_id.as_u64())
			.ok_or_else(govanerror::debug_lazy!(
				log fmt = ("Reaction in unavailable guild {}", this_channel.guild_id)
			))?;

		if server
			.channels
			.disallowed_listen
			.contains(&this_channel.id.into())
		{
			return Err(govanerror::debug!(
				// log = "Reaction in unavailable channel"
			));
		}

		let reactor = add_reaction.user(&ctx).await?;

		let author = if reactor.id == ctx.cache.current_user_id() {
			"I".to_string()
		} else {
			reactor.name.to_string()
		};

		logger::info_fmt!(
			"{} reacted on {} @ {}: {}",
			author.cyan(),
			add_reaction.message_id,
			this_channel.name,
			add_reaction.emoji.to_string()
		);

		if reactor.id == ctx.cache.current_user_id() {
			return Err(govanerror::debug!(log = "No dispatching reactions on self"));
		}

		let msg = add_reaction.message(&ctx.http).await?;

		msg.guild_cached(ctx).await?;

		enum DarkSoulsType {
			FireHeart,
			Headstone,
		}

		enum Action {
			DarkSouls(DarkSoulsType),
			Retweet {
				with_context: bool,
				verified_role: Option<u64>,
			},
			Pin {
				destination_id: u64,
				required: usize,
				emoji_override: Option<EmojiType>,
			},
			AlwaysSunny,
			None,
		}

		let action = {
			let emoji: EmojiType = EmojiType::from(&add_reaction.emoji);
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
							if emoji.contains(crate::data::emoji::PIN) {
								Some(crate::data::emoji::REDDIT_GOLD)
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
			} else {
				// One-offs
				let action = match emoji {
					EmojiType::Unicode(ref code) => match code.as_str() {
						data::emoji::FIRE_HEART => Action::DarkSouls(DarkSoulsType::FireHeart),
						data::emoji::HEADSTONE => Action::DarkSouls(DarkSoulsType::Headstone),
						data::emoji::REPEAT => Action::Retweet {
							with_context: true,
							verified_role: server.no_context.as_ref().map(|c| c.role),
						},
						data::emoji::REPEAT_ONCE => Action::Retweet {
							with_context: false,
							verified_role: server.no_context.as_ref().map(|c| c.role),
						},
						data::emoji::VIOLIN => Action::AlwaysSunny,
						_ => Action::None,
					},
					EmojiType::Discord(_) => Action::None,
				};

				// Hall-of-all
				if matches!(action, Action::None) {
					if let Some(hall_of_all) = server.hall_of_all.as_ref() {
						let channel_id = hall_of_all.channel;

						Action::Pin {
							destination_id: channel_id,
							required,
							emoji_override: None,
						}
					} else {
						action
					}
				} else {
					action
				}
			}
		};

		match action {
			Action::None => Ok(()),
			Action::DarkSouls(souls_type) => {
				use crate::helpers::text_banners;

				if msg.content.is_empty() {
					return Err(govanerror::error!(
						log = "Attempting to donk bonk empty message",
						user = "That message has no text for me to use!"
					));
				}

				{
					let pin_lock = self.pin_lock().await;
					pin_lock
						.locked_react(
							ctx,
							msg.id,
							msg.channel_id,
							add_reaction,
							None,
							Some(std::time::Duration::from_secs(60 * 30)),
						)
						.await?;
				}

				let preset = match souls_type {
					DarkSoulsType::Headstone => text_banners::Preset::YOU_DIED.clone(),
					DarkSoulsType::FireHeart => {
						if util::random::one_in(100) {
							text_banners::Preset {
								main_color: text_banners::Rgb(
									util::random::from_range(50..255),
									util::random::from_range(50..255),
									util::random::from_range(50..255),
								),

								sheen_tint: text_banners::Rgb(
									util::random::from_range(50..255),
									util::random::from_range(50..255),
									util::random::from_range(50..255),
								),

								text_spacing: util::random::from_range(0..20) as f32,
								sheen_size: util::random::from_range(0.0..2.0),
								sheen_opacity: util::random::from_range(0.0..0.2),
								text_opacity: None,
								shadow_opacity: None,
								font: text_banners::Font::Garamond,
								font_weight: None,
							}
						} else {
							util::random::pick(&[
								text_banners::Preset::BONFIRE_LIT,
								text_banners::Preset::HUMANITY_RESTORED,
								text_banners::Preset::VICTORY_ACHIEVED,
							])
							.unwrap()
							.clone()
						}
					}
				};

				let gradient = if util::random::one_in(100) {
					util::random::pick(&[
						text_banners::gradients::LGBT,
						text_banners::gradients::TRANS,
						text_banners::gradients::BI,
						text_banners::gradients::LESBIAN,
						text_banners::gradients::ENBI,
						text_banners::gradients::PAN,
					])
					.map(|g| g.to_owned())
				} else {
					None
				};

				let data = text_banners::create_image(&msg.content, &preset, gradient).await;

				this_channel
					.send_message(&ctx, |b| {
						b.add_file((
							data.as_bytes(),
							format!("donk_blonk_{}.png", reactor.name).as_str(),
						))
					})
					.await?;
				Ok(())
			}
			Action::Retweet {
				with_context,
				verified_role,
			} => {
				{
					let pin_lock = self.pin_lock().await;
					pin_lock
						.locked_react(
							ctx,
							msg.id,
							msg.channel_id,
							add_reaction,
							None,
							Some(std::time::Duration::from_secs(60 * 30)),
						)
						.await?;
				}

				self.maybe_retweet(ctx, &msg, add_reaction, with_context, verified_role)
					.await?;
				Ok(())
			}
			Action::AlwaysSunny => {
				{
					let pin_lock = self.pin_lock().await;
					pin_lock
						.locked_react(
							ctx,
							msg.id,
							msg.channel_id,
							add_reaction,
							None,
							Some(std::time::Duration::from_secs(60 * 30)),
						)
						.await?;
				}

				self.maybe_iasip(ctx, &msg).await?;
				Ok(())
			}
			Action::Pin {
				destination_id,
				required,
				emoji_override,
			} => {
				// No pinning your own messages, bot
				if msg.is_own(ctx) {
					return Err(govanerror::debug!(log = "Won't pin myself"));
				}

				{
					let pin_lock = self.pin_lock().await;
					pin_lock
						.locked_react(
							ctx,
							msg.id,
							msg.channel_id,
							add_reaction,
							Some(required),
							None,
						)
						.await?;
				}
				let channel = ChannelId(destination_id).to_channel(&ctx).await?;
				let channel = channel.guild().ok_or_else(govanerror::error_lazy!(
				  log fmt = ("Channel {} is misconfigured with {:?} pin", destination_id, emoji_override),
				  user = "< This guy's creator has fucked up"
				))?;

				channel
					.guild_cached(ctx)
					.await
					.map_err(|e| e.with_user_string_weak("Oh no, problems"))?;

				self.maybe_pin(ctx, msg, add_reaction, channel, emoji_override)
					.await?;

				Ok(())
			}
		}
	}
}
