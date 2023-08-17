use std::convert::Infallible;

use crate::bot::data::EmojiType;
use crate::bot::Bot;

use serenity::builder::CreateMessage;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::util::{self, logger, ResultErrorHandler};
use rand::distributions::Uniform;
use rand::prelude::*;

impl Bot {
	pub async fn maybe_pin(
		&self,
		ctx: Context,
		msg: Message,
		reaction: Reaction,
		dest: GuildChannel,
		required: usize,
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
				.locked_react(&ctx, &msg, &reaction, Some(required), None)
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
