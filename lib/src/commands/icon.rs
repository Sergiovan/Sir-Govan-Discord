use crate::prelude::*;

use super::commander::Arguments;
use crate::bot::Bot;
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
) -> GovanResult {
	let guild_id = msg.guild_id.ok_or_else(govanerror::debug_lazy!(
		log = "Command used outside of guild",
		user = "You need to be in a guild, silly!"
	))?;

	let guild = guild_id.to_partial_guild(&ctx).await?;

	let tier = guild.premium_tier;

	match tier {
		PremiumTier::Tier0 | PremiumTier::Tier1 => {
			return Err(govanerror::debug!(
				log = "Guild does not have access to this function",
				user = "This guild is not Tier 2 or higher, so I can't set your icon"
			));
		}
		_ => (),
	}

	let member = msg.member(&ctx).await?;

	let role = member.get_unique_role(ctx)?;

	use super::commander::Argument;
	use data::EmojiType;

	let arg = words.arg();
	if let Some(Argument::Emoji(EmojiType::Discord(icon))) = arg {
		let emoji_id = icon;
		let icon = util::url_from_discord_emoji(icon, false);

		role.set_icon(ctx, guild_id, &icon).await?;

		msg.reply_report(ctx, &format!("Icon set. Enjoy your <:emoji:{}>", emoji_id))
			.await;
	} else if let Some(Argument::Emoji(EmojiType::Unicode(icon))) = arg {
		role.set_unicode_icon(ctx, guild_id, &icon).await?;

		msg.reply_report(ctx, &format!("Icon set. Enjoy your {}", icon))
			.await
	} else if let Some(Argument::String(icon)) = arg {
		role.set_icon(ctx, guild_id, icon).await?;

		msg.reply_report(ctx, "Icon set. Enjoy").await;
	} else if arg.is_none() {
		role.reset_icon(ctx, guild_id).await?;

		msg.reply_report(ctx, "Icon reset. Woo").await;
	}

	Ok(())
}
