use crate::prelude::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

use super::commander::{Arguments, CommandResult};
use crate::bot::Bot;

use sirgovan_macros::command;

#[command]
async fn ping<'a>(
	&self,
	ctx: &Context,
	msg: &'a Message,
	mut _words: Arguments<'a>,
	bot: &Bot,
) -> CommandResult {
	msg.reply_report(ctx, bot.data.read().await.strings.ping.pick())
		.await;

	Ok(())
}
