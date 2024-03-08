use crate::prelude::*;

use serenity::model::prelude::*;
use serenity::prelude::*;

use super::commander::Arguments;
use crate::bot::Bot;
use crate::prelude::MessageExt;

use sirgovan_macros::command;

#[command]
async fn quit<'a>(
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

	let param = words.string();

	if param.is_none() || (param.is_some_and(|p| p == "beta") && bot.data().await.beta) {
		msg.reply_report(ctx, "Bye!").await;
		bot.shutdown().await;
	} else {
		msg.reply_report(ctx, "You're looking for [beta]").await;
	}

	Ok(())
}
