use serenity::model::prelude::*;
use serenity::prelude::*;

use async_trait::async_trait;

use super::commander::Arguments;
use crate::bot::Bot;
use crate::commands::commander::Command;

use std::convert::Infallible;

use sirgovan_macros::command;

#[command]
fn quit<'a>(
	&self,
	_ctx: &Context,
	msg: &'a Message,
	mut _words: Arguments<'a>,
	bot: &Bot,
) -> Option<Infallible> {
	if msg.author.id == 120881455663415296 {
		bot.shutdown().await;
	}

	None
}
