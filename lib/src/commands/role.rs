use crate::prelude::*;

use super::commander::{Arguments, CommandResult};
use crate::bot::Bot;
use serenity::model::prelude::*;
use serenity::prelude::*;

use sirgovan_macros::command;

#[derive(thiserror::Error, Debug)]
enum RoleError {
	#[error("")]
	NotInGuild,
	#[error("Guild not in server list: {0}")]
	GuildNotInList(GuildId),
	#[error("")]
	GuildMissingRole,
	#[error("Could not get role name for {0}")]
	RoleNoName(u64),
}

impl Reportable for RoleError {
	fn to_user(&self) -> Option<String> {
		match self {
			Self::RoleNoName(..) => {
				Some("We're backlogged, please try again in 5 business years".into())
			}
			_ => None,
		}
	}
}

#[command]
async fn role<'a>(
	&self,
	ctx: &Context,
	msg: &'a Message,
	mut _words: Arguments<'a>,
	bot: &Bot,
) -> CommandResult<RoleError> {
	let guild_id = msg.guild_id.ok_or(RoleError::NotInGuild)?;

	let bot_data = bot.data.read().await;
	let server = bot_data
		.servers
		.get(&guild_id.into())
		.ok_or(RoleError::GuildNotInList(guild_id))?;

	let no_context = server
		.no_context
		.as_ref()
		.ok_or(RoleError::GuildMissingRole)?;

	let role_name = RoleId(no_context.role)
		.to_role_cached(ctx)
		.ok_or(RoleError::RoleNoName(no_context.role))?;

	let (number, out_of) = bot_data.no_context_index(&role_name.name);

	if let Some(number) = number {
		msg.reply_report(
			ctx,
			&format!("{}/{}: {}", number + 1, out_of, role_name.name),
		)
		.await;
	} else {
		msg.reply_report(
			ctx,
			&format!("NaN: {}\nThis role is shiny!", role_name.name),
		)
		.await;
	}

	Ok(())
}
