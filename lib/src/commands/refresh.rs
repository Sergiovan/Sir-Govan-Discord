use crate::prelude::*;

use serenity::model::prelude::*;
use serenity::prelude::*;

use super::commander::Arguments;
use crate::bot::Bot;

use sirgovan_macros::command;

#[command]
async fn refresh<'a>(
	&self,
	ctx: &Context,
	msg: &'a Message,
	mut words: Arguments<'a>,
	bot: &Bot,
) -> GovanResult {
	if msg.author.id != 120881455663415296 {
		return Err(govanerror::error!(
			log fmt = ("Attempted illegal !quit by non-owner: {}", msg.author.name),
			user = "Nuh-uh"
		));
	}

	let mut bot_data = bot.data.write().await;
	let mut servers_res = None;
	let mut no_context_res = None;
	let mut strings_res = None;

	match words.string() {
		Some("all") => {
			servers_res = Some(bot_data.load_servers());
			no_context_res = Some(bot_data.load_role_names());
			strings_res = Some(bot_data.load_strings());
		}
		Some("servers") => {
			servers_res = Some(bot_data.load_servers());
		}
		Some("roles") => {
			no_context_res = Some(bot_data.load_role_names());
		}
		Some("strings") => {
			strings_res = Some(bot_data.load_strings());
		}
		Some(_) | None => {
			msg.reply_report(ctx, "You want [all|servers|roles|strings]")
				.await;
			return Ok(());
		}
	}

	let mut problems = Vec::with_capacity(3);

	if let Some(Err(e)) = servers_res {
		e.log();
		problems.push("servers");
	}

	if let Some(Err(e)) = no_context_res {
		e.log();
		problems.push("roles");
	}

	if let Some(Err(e)) = strings_res {
		e.log();
		problems.push("strings");
	}

	if problems.is_empty() {
		msg.reply_report(ctx, "All done!").await;
	} else {
		msg.reply_report(
			ctx,
			&format!("Problems found while refreshing: {}", problems.join(", ")),
		)
		.await;
	}

	Ok(())
}
