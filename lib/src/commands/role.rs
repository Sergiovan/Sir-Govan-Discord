use crate::prelude::*;

use super::commander::Arguments;
use crate::bot::Bot;
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
) -> GovanResult {
	let guild_id = msg.guild_id.ok_or_else(govanerror::debug_lazy!(
		log = "Command used outside of guild",
		user = "You need to be in a guild, silly!"
	))?;

	let bot_data = bot.data().await;
	let server = bot_data
		.servers
		.get(&guild_id.into())
		.ok_or_else(govanerror::error_lazy!(
			log fmt = ("Guild {} not in server list", guild_id),
			user = "< This guy's maker is a doofus"
		))?;

	let no_context = server
		.no_context
		.as_ref()
		.ok_or_else(govanerror::debug_lazy!(
			log = "Guild does not have nocontext role",
			user = "This guild does not have a role to keep track of"
		))?;

	if no_context.role == 0 {
		return Err(govanerror::error!(
			log fmt = ("No context role for {} is 0", guild_id),
			user = "< This guy's maker is a doofus"
		));
	}

	let role_name = util::role_from_id(RoleId::new(no_context.role), ctx).ok_or_else(
		govanerror::error_lazy!(
			log fmt = ("Role {} was not cached properly", no_context.role),
			user = "Discord is being silly again. Try again later"
		),
	)?;

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
