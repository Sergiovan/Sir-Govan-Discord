use std::ops::Not;

use crate::prelude::*;

use crate::bot::Bot;
use crate::data::EmojiType;

use serenity::builder::CreateMessage;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::util::random;

#[derive(thiserror::Error, Debug)]
pub enum HallError {
	#[error("Generic error: {0}")]
	GenericError(#[from] anyhow::Error),
	#[error("Discord api error: {0}")]
	DiscordError(#[from] serenity::Error),
	#[error("No permission to post in hall {0}")]
	NoPermission(String),
}

impl Reportable for HallError {}

impl Bot {
	pub async fn maybe_pin(
		&self,
		ctx: &Context,
		msg: Message,
		reaction: &Reaction,
		dest: GuildChannel,
		override_icon: Option<EmojiType>,
	) -> GovanResult {
		let perms = dest.permissions_for_user(ctx, ctx.cache.current_user())?;

		if !perms.send_messages() {
			return Err(govanerror::error!(
				log fmt = ("Channel misconfigured: No permission to post in {}", dest.name),
				user = "< This guy's creator is a foolish human"
			));
		}

		const FALLBACK: &str = "https://twemoji.maxcdn.com/v/latest/72x72/2049.png";

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
			r: random::from_range(0..0x10) * 0x10,
			g: random::from_range(0..0x10) * 0x10,
			b: random::from_range(0..0x10) * 0x10,
			message_url: msg.link(),
			author: msg.author.name.clone(),
			author_avatar: msg
				.author
				.avatar_url()
				.clone()
				.unwrap_or(msg.author.default_avatar_url()),
			content: msg.content.is_empty().not().then_some(msg.content),
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
			.await?;

		Ok(())
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
