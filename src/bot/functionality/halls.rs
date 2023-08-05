use std::convert::Infallible;

use crate::bot::data::EmojiType;
use crate::bot::Bot;

use serenity::builder::CreateMessage;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::util::{self, logger, ResultErrorHandler};
use rand::distributions::Uniform;
use rand::prelude::*;
pub struct HallSafety;

impl HallSafety {
	async fn get_reactors(
		&self,
		ctx: &Context,
		msg: &Message,
		reaction: &Reaction,
		required: u32,
	) -> Vec<User> {
		// First get the emoji for sure
		if let ReactionType::Custom { name: None, .. } = reaction.emoji {
			logger::error(&format!(
				"Emoji from reaction was incomplete: {}",
				reaction.emoji
			));
			return vec![];
		};

		let mut last: Option<UserId> = None;
		let mut res = vec![];

		loop {
			// NOTE: Unknown how `last` interacts with the order in which reaction_users are returned
			match msg
				.reaction_users(&ctx, reaction.emoji.clone(), None, last)
				.await
			{
				Ok(users) => {
					let filtered = users
						.into_iter()
						.filter(|x| !x.bot && x.id != msg.author.id)
						.collect::<Vec<_>>();

					if filtered.is_empty() {
						return res;
					}

					res.extend(filtered);
				}
				Err(e) => {
					logger::error(&format!(
						"Could not get {} reactions from {}: {}",
						reaction.emoji, msg.id, e
					));
					return vec![];
				}
			};

			if res.len() > required as usize {
				return res;
			}

			last = res.last().map(|x| x.id);
		}
	}

	pub async fn locked_react(
		&self,
		ctx: &Context,
		msg: &Message,
		reaction: &Reaction,
		required: u32,
	) -> bool {
		// The only way to access this function is by locking HallSafety, so we're, well, safe

		let Some(msg_reactions) = msg
            .reactions
            .iter()
            .find(|x| x.reaction_type == reaction.emoji)
        else {
            return false; // No reactions to speak of, cannot pin
        };

		if msg_reactions.me {
			return false; // No reactions if I've already reacted
		}

		let reactors = self.get_reactors(ctx, msg, reaction, required).await;

		if reactors.len() >= required as usize {
			match msg.react(&ctx, reaction.emoji.clone()).await {
				Ok(_) => true,
				Err(e) => {
					logger::error(&format!(
						"Error while adding {} reaction to {}: {}",
						reaction.emoji, msg.id, e
					));
					false
				}
			}
		} else {
			false
		}
	}
}

impl Bot {
	pub async fn maybe_pin(
		&self,
		ctx: Context,
		msg: Message,
		reaction: Reaction,
		dest: GuildChannel,
		required: u32,
		override_icon: Option<EmojiType>,
	) -> Option<Infallible> {
		let perms = dest
			.permissions_for_user(&ctx, ctx.cache.current_user())
			.unwrap_or_log(&format!(
				"Could not get permissions for self in {}",
				dest.name
			))?;

		if !perms.send_messages() {
			logger::error(&format!(
				"Cannot pin to {}: No permission to send messages",
				dest.name
			));
			return None; // Unspeakable channel
		}

		let can_pin = {
			let hall_safety = self.pin_lock.lock().await;
			hall_safety
				.locked_react(&ctx, &msg, &reaction, required)
				.await
		};

		if can_pin {
			const FALLBACK: &str = "https://twemoji.maxcdn.com/v/latest/72x72/2049.png";
			let color_range = Uniform::from(0..0x10);

			let pin_data = PinData {
				icon_url: if let Some(emoji) = override_icon {
					match emoji {
						EmojiType::Unicode(ref emoji) => util::url_from_unicode_emoji(emoji),
						EmojiType::Discord(id) => util::url_from_discord_emoji(id, false),
					}
				} else {
					match reaction.emoji {
						ReactionType::Unicode(ref emoji) => util::url_from_unicode_emoji(emoji),
						ReactionType::Custom { animated, id, .. } => {
							util::url_from_discord_emoji(id.into(), animated)
						}
						_ => FALLBACK.to_string(),
					}
				},
				r: color_range.sample(&mut rand::thread_rng()) * 0x10, // TODO Use own random number generator
				g: color_range.sample(&mut rand::thread_rng()) * 0x10,
				b: color_range.sample(&mut rand::thread_rng()) * 0x10,
				message_url: msg.link(),
				author: msg.author.name.clone(),
				author_avatar: msg
					.author
					.avatar_url()
					.clone()
					.unwrap_or(msg.author.default_avatar_url()),
				content: if msg.content.is_empty() {
					None
				} else {
					Some(msg.content)
				},
				timestamp: msg.timestamp,
				message_id: *msg.id.as_u64(),
				channel_id: *msg.channel_id.as_u64(),
				embed: if let [ref first, ..] = &msg.attachments[..] {
					let content_type = first.content_type.as_ref();
					if content_type.is_some_and(|x| x.starts_with("video/")) {
						Embed::Video(first.filename.clone())
					} else if content_type.is_some_and(|x| x.starts_with("audio/")) {
						Embed::Audio(first.filename.clone())
					} else {
						Embed::Image(first.url.clone())
					}
				} else if let [ref first, ..] = &msg.embeds[..] {
					match first.image.as_ref() {
						Some(url) => Embed::Image(url.url.clone()),
						None => match first.thumbnail.as_ref() {
							Some(thumb) => Embed::Image(thumb.url.clone()),
							None => Embed::Nothing,
						},
					}
				} else if let [ref first, ..] = &msg.sticker_items[..] {
					first
						.image_url()
						.map_or_else(|| Embed::Nothing, Embed::Image)
				} else {
					Embed::Nothing
				},
			};
			dest.send_message(&ctx, |b| self.make_pin(b, pin_data))
				.await
				.log_if_err(&format!(
					"Error while sending pin of {} to {}",
					msg.id, dest.name
				));
		};

		None
	}

	fn make_pin<'a, 'b>(
		&self,
		builder: &'a mut CreateMessage<'b>,
		data: PinData,
	) -> &'a mut CreateMessage<'b> {
		builder.add_embed(|b| {
			b.color(data.r << 16 | data.g << 8 | data.b)
				.author(|b| b.name(data.author).icon_url(data.author_avatar))
				.timestamp(data.timestamp)
				.footer(|b| {
					b.text(format!("{} - {}", data.message_id, data.channel_id))
						.icon_url(data.icon_url)
				});

			let teleport = match data.embed {
				Embed::Image(url) => {
					b.image(url);
					format!("[Click to teleport]({})", data.message_url)
				}
				Embed::Video(name) => format!("[Click to go watch {}]({})", name, data.message_url),
				Embed::Audio(name) => {
					format!("[Click to go listen to {}]({})", name, data.message_url)
				}
				Embed::Nothing => format!("[Click to teleport]({})", data.message_url),
			};

			if let Some(content) = data.content {
				b.description(content);
				b.field("\u{200b}", teleport, false);
			} else {
				b.description(teleport);
			}

			b
		})
	}
}

enum Embed {
	Image(String),
	Video(String),
	Audio(String),
	Nothing,
}

struct PinData {
	icon_url: String,
	r: u32,
	g: u32,
	b: u32,
	message_url: String,
	author: String,
	author_avatar: String,
	content: Option<String>,
	timestamp: Timestamp,
	message_id: u64,
	channel_id: u64,
	embed: Embed,
}
