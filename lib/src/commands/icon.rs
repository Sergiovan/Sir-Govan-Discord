use crate::prelude::*;

use std::convert::Infallible;

use super::commander::{Arguments, Command};
use crate::bot::Bot;
use async_trait::async_trait;
use image::EncodableLayout;
use serenity::model::prelude::*;
use serenity::prelude::*;

use sirgovan_macros::command;

#[command]
async fn icon<'a>(
	&self,
	ctx: &Context,
	msg: &'a Message,
	mut words: Arguments<'a>,
	_bot: &Bot,
) -> Option<Infallible> {
	let guild_id = msg.guild_id?;

	let guild = guild_id
		.to_partial_guild(&ctx)
		.await
		.ok_or_log(&format!("Could not get guild from id {}", guild_id))?;

	let tier = guild.premium_tier;

	match tier {
		PremiumTier::Tier0 | PremiumTier::Tier1 => {
			msg.reply_report(ctx, "This guild does not have access to this feature!")
				.await;
			return None;
		}
		_ => (),
	}

	let member = msg
		.member(&ctx)
		.await
		.ok_or_log(&format!("Could not get member from {}", msg.author.id))?;
	let role = match member.get_unique_role(ctx) {
		Ok(role) => role,
		Err(e) => {
			e.get_messages().report(ctx, msg).await;
			return None;
		}
	};

	async fn set_icon(
		ctx: &Context,
		role: &Role,
		guild_id: GuildId,
		value: &str,
	) -> anyhow::Result<()> {
		let url = reqwest::Url::parse(value)?;

		let bytes = reqwest::get(url).await?.bytes().await?;
		let bytes = match image::guess_format(&bytes) {
			Ok(image::ImageFormat::Png) => bytes.into_iter().collect::<Vec<_>>(),
			_ => {
				use image::GenericImageView;
				use image::ImageEncoder;

				let buffer = image::load_from_memory(bytes.as_bytes())?;
				let (w, h) = buffer.dimensions();

				let mut png = Vec::new();
				let encoder = image::codecs::png::PngEncoder::new(&mut png);

				encoder.write_image(buffer.as_bytes(), w, h, buffer.color())?;

				png
			}
		};

		// if url.is_err() {
		// 	msg.reply_report(ctx, "That is not a valid url or emoji")
		// 		.await;
		// };

		let mut encoded = openssl::base64::encode_block(&bytes);
		encoded.insert_str(0, "data:image/png;base64,");

		// I do it like this because `.icon` is async so I can't use it inside an `.edit_role` lambda
		let mut edit_role = serenity::builder::EditRole::new(role);

		edit_role
			.0
			.insert("unicode_emoji", serenity::json::Value::Null);
		edit_role.0.insert("icon", encoded.into());
		// {
		// 	logger::error(&format!("Could not gather data from url {}: {}", value, e));

		// 	msg.reply_report(ctx, "I had trouble getting image data from that url")
		// 		.await;
		// 	return None;
		// }

		let map = serenity::json::hashmap_to_json_map(edit_role.0);

		ctx.http
			.as_ref()
			.edit_role(guild_id.into(), role.id.into(), &map, None)
			.await
			.map(|_| ())
			.map_err(|e| e.into())
	}

	async fn set_unicode_icon(
		ctx: &Context,
		role: &Role,
		guild_id: GuildId,
		value: &str,
	) -> anyhow::Result<()> {
		let mut edit_role = serenity::builder::EditRole::new(role);

		edit_role.0.insert("unicode_emoji", value.into());
		edit_role.0.insert("icon", serenity::json::Value::Null);

		let map = serenity::json::hashmap_to_json_map(edit_role.0);

		ctx.http
			.as_ref()
			.edit_role(guild_id.into(), role.id.into(), &map, None)
			.await
			.map(|_| ())
			.map_err(|e| e.into())
	}

	async fn reset_icon(ctx: &Context, role: &Role, guild_id: GuildId) -> anyhow::Result<()> {
		// I do it like this because there's no other way lmfao
		let mut edit_role = serenity::builder::EditRole::new(role);
		edit_role
			.0
			.insert("unicode_emoji", serenity::json::Value::Null);
		edit_role.0.insert("icon", serenity::json::Value::Null);

		let map = serenity::json::hashmap_to_json_map(edit_role.0);

		ctx.http
			.as_ref()
			.edit_role(guild_id.into(), role.id.into(), &map, None)
			.await
			.map(|_| ())
			.map_err(|e| e.into())
	}

	use super::commander::Argument;
	use data::EmojiType;

	let arg = words.arg();
	if let Some(Argument::Emoji(EmojiType::Discord(icon))) = arg {
		let emoji_id = icon;
		let icon = util::url_from_discord_emoji(icon, false);
		if let Err(e) = set_icon(ctx, &role, guild_id, &icon).await {
			logger::error(&format!(
				"Could not set icon of {} to image from {}: {}",
				role.id, icon, e
			));
			msg.reply_report(ctx, "I'm having trouble setting your icon to that")
				.await;
			return None;
		} else {
			msg.reply_report(ctx, &format!("Icon set. Enjoy your <:emoji:{}>", emoji_id))
				.await;
		}
	} else if let Some(Argument::Emoji(EmojiType::Unicode(icon))) = arg {
		match set_unicode_icon(ctx, &role, guild_id, &icon).await {
			Ok(..) => {
				msg.reply_report(ctx, &format!("Icon changed. Enjoy your {}", icon))
					.await
			}
			Err(e) => {
				logger::error(&format!(
					"Unable to reset icon for {}: {}",
					member.user.id, e
				));
				msg.reply_report(ctx, "Could not reset your icon, it is too powerful")
					.await;
			}
		};
	} else if let Some(Argument::String(icon)) = arg {
		if let Err(e) = set_icon(ctx, &role, guild_id, icon).await {
			logger::error(&format!(
				"Could not set icon of {} to image from {}: {}",
				role.id, icon, e
			));
			msg.reply_report(ctx, "I'm having trouble setting your icon to that")
				.await;
			return None;
		} else {
			msg.reply_report(ctx, "Icon set. Enjoy").await;
		}
	} else if arg.is_none() {
		match reset_icon(ctx, &role, guild_id).await {
			Ok(..) => msg.reply_report(ctx, "Icon reset. Woo").await,
			Err(e) => {
				logger::error(&format!(
					"Unable to reset icon for {}: {}",
					member.user.id, e
				));
				msg.reply_report(ctx, "Could not reset your icon, it is too powerful")
					.await;
			}
		};
	}

	None
}
