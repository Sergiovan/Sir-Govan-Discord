use crate::prelude::*;

use super::commander::{Arguments, CommandResult};
use crate::bot::Bot;
use serenity::model::prelude::*;
use serenity::prelude::*;
use util::traits::SetIconError;

use sirgovan_macros::command;

#[derive(thiserror::Error, Debug)]
enum IconError {
	#[error("")]
	NotInGuild,
	#[error("could not get guild from id {0}: {1}")]
	GuildNotInList(GuildId, #[source] anyhow::Error),
	#[error("")]
	GuildNotPremium,
	#[error("could not get member from {0}: {1}")]
	MemberFailure(UserId, #[source] anyhow::Error),
	#[error("{0}")]
	UniqueRoleError(#[from] util::traits::UniqueRoleError),
	#[error("could not set icon of {1} to image from {2}: {0}")]
	IconSetError(#[source] SetIconError, RoleId, String),
	#[error("could not reset icon for {1}: {0}")]
	IconResetError(#[source] SetIconError, UserId),
}

impl Reportable for IconError {
	fn to_user(&self) -> Option<String> {
		match self {
			Self::GuildNotPremium => Some("This guild does not have access to this feature".into()),
			Self::MemberFailure(..) => {
				Some("The Discord API is being funny, please try again later".into())
			}
			Self::UniqueRoleError(e) => e.to_user(),
			Self::IconSetError(e, ..) => match e {
				SetIconError::UrlParseError(..) => Some("The url given is invalid".into()),
				SetIconError::ImageError(..) => Some("I cannot handle this image".into()),
				_ => Some("I'm having trouble setting your icon to that".into()),
			},
			Self::IconResetError(..) => {
				Some("Could not reset your icon, it is too powerful".into())
			}
			_ => None,
		}
	}
}

#[command]
async fn icon<'a>(
	&self,
	ctx: &Context,
	msg: &'a Message,
	mut words: Arguments<'a>,
	_bot: &Bot,
) -> CommandResult<IconError> {
	let guild_id = msg.guild_id.ok_or(IconError::NotInGuild)?;

	let guild = guild_id
		.to_partial_guild(&ctx)
		.await
		.map_err(|e| IconError::GuildNotInList(guild_id, e.into()))?;

	let tier = guild.premium_tier;

	match tier {
		PremiumTier::Tier0 | PremiumTier::Tier1 => {
			return Err(IconError::GuildNotPremium);
		}
		_ => (),
	}

	let member = msg
		.member(&ctx)
		.await
		.map_err(|e| IconError::MemberFailure(msg.author.id, e.into()))?;

	let role = member.get_unique_role(ctx)?;

	use super::commander::Argument;
	use data::EmojiType;

	let arg = words.arg();
	if let Some(Argument::Emoji(EmojiType::Discord(icon))) = arg {
		let emoji_id = icon;
		let icon = util::url_from_discord_emoji(icon, false);

		role.set_icon(ctx, guild_id, &icon)
			.await
			.map_err(|e| IconError::IconSetError(e, role.id, icon))?;

		msg.reply_report(ctx, &format!("Icon set. Enjoy your <:emoji:{}>", emoji_id))
			.await;
	} else if let Some(Argument::Emoji(EmojiType::Unicode(icon))) = arg {
		role.set_unicode_icon(ctx, guild_id, &icon)
			.await
			.map_err(|e| IconError::IconSetError(e, role.id, icon.clone()))?;

		msg.reply_report(ctx, &format!("Icon set. Enjoy your {}", icon))
			.await
	} else if let Some(Argument::String(icon)) = arg {
		role.set_icon(ctx, guild_id, icon)
			.await
			.map_err(|e| IconError::IconSetError(e, role.id, icon.to_string()))?;

		msg.reply_report(ctx, "Icon set. Enjoy").await;
	} else if arg.is_none() {
		role.reset_icon(ctx, guild_id)
			.await
			.map_err(|e| IconError::IconResetError(e, member.user.id))?;

		msg.reply_report(ctx, "Icon reset. Woo").await;
	}

	Ok(())
}
