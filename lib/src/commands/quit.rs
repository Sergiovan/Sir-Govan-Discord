use serenity::model::prelude::*;
use serenity::prelude::*;

use super::commander::Arguments;
use crate::bot::Bot;

use sirgovan_macros::command;

#[command]
async fn quit<'a>(
	&self,
	_ctx: &Context,
	msg: &'a Message,
	mut _words: Arguments<'a>,
	bot: &Bot,
) -> GovanResult {
	if msg.author.id == 120881455663415296 {
		bot.shutdown().await;
	}

	Ok(())
}
