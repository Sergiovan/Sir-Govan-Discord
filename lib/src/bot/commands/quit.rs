use serenity::model::prelude::*;
use serenity::{async_trait, prelude::*};

use super::commander::Arguments;
use crate::bot::commands::commander::Command;
use crate::bot::Bot;

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
