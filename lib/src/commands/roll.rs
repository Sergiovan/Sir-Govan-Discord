use crate::prelude::*;
use crate::util::random::{self, RandomBag};
use num_bigint::BigInt;
use serenity::model::prelude::*;
use serenity::prelude::*;

use super::commander::{Arguments, CommandResult};
use crate::bot::Bot;

use sirgovan_macros::command;

#[derive(thiserror::Error, Debug)]
enum RollError {
	#[error("")]
	NegativeRoll,
}

impl Reportable for RollError {
	fn to_user(&self) -> Option<String> {
		match self {
			Self::NegativeRoll => Some(
				"Dice need to have 1 or more sides, otherwise I don't know where the North is"
					.into(),
			),
		}
	}
}

#[command]
async fn roll<'a>(
	&self,
	ctx: &Context,
	msg: &'a Message,
	mut words: Arguments<'a>,
	bot: &Bot,
) -> CommandResult<RollError> {
	let number = words.big_number().unwrap_or(20.into());

	if number <= 0.into() {
		return Err(RollError::NegativeRoll);
	}

	let rand = random::from_range(BigInt::from(1_u64)..=number);

	msg.reply_report(
		ctx,
		&format!("{}{}", rand, bot.data.read().await.strings.roll.pick()),
	)
	.await;

	Ok(())
}
