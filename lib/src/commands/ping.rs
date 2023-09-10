use crate::prelude::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

use super::commander::Arguments;
use crate::bot::Bot;

use sirgovan_macros::command;

#[command]
async fn ping<'a>(
	&self,
	ctx: &Context,
	msg: &'a Message,
	mut _words: Arguments<'a>,
	bot: &Bot,
) -> GovanResult {
	msg.reply_report(ctx, bot.data().await.strings.ping.pick())
		.await;

	Ok(())
}
