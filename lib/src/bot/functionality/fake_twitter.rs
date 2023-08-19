use std::convert::Infallible;

use crate::bot::Bot;

use serenity::model::prelude::*;
use serenity::prelude::*;

impl Bot {
	pub async fn maybe_retweet(
		&self,
		ctx: Context,
		msg: Message,
		with_context: bool,
	) -> Option<Infallible> {
		None
	}
}
