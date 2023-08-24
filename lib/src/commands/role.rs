use crate::prelude::*;

use std::convert::Infallible;

use super::commander::{Arguments, Command};
use crate::bot::Bot;
use async_trait::async_trait;
use serenity::model::prelude::*;
use serenity::prelude::*;

use sirgovan_macros::command;

#[command]
async fn role<'a>(
	&self,
	ctx: &Context,
	msg: &'a Message,
	mut _words: Arguments<'a>,
	bot: &Bot,
) -> Option<Infallible> {
	let guild_id = msg.guild_id?;

	let bot_data = bot.data.read().await;
	let server = bot_data
		.servers
		.get(&guild_id.into())
		.log_if_none(&format!("Guild not in server list {}", guild_id))?;

	let no_context = server.no_context.as_ref()?;

	let role_name = RoleId(no_context.role).to_role_cached(ctx);

	let Some(role_name) = role_name.log_if_none(&format!("Could not get role name from {}", no_context.role)) else {
    msg.reply_report(ctx, "We're backlogged, please try again in 5 business years").await;
    return None;
  };

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

	None
}
