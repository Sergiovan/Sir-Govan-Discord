use crate::prelude::*;
use crate::util::random::{self, RandomBag};
use num_bigint::BigInt;
use serenity::model::prelude::*;
use serenity::prelude::*;

use super::commander::Arguments;
use crate::bot::Bot;

use sirgovan_macros::command;

#[command]
async fn roll<'a>(
	&self,
	ctx: &Context,
	msg: &'a Message,
	mut words: Arguments<'a>,
	bot: &Bot,
) -> GovanResult {
	let number = words.big_number().unwrap_or(20.into());

	if number <= 0.into() {
		return Err(govanerror::debug!(
			log fmt = ("Attempted negative roll: {}", number),
			user = "Dice need to have 1 or more sides, otherwise I don't know where the North is"
		));
	}

	let rand = random::from_range(BigInt::from(1_u64)..=number);

	msg.reply_report(
		ctx,
		&format!("{}{}", rand, bot.data.read().await.strings.roll.pick()),
	)
	.await;

	Ok(())
}
