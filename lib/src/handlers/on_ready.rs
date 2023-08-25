use std::convert::Infallible;

use crate::bot::Bot;
use crate::util::logger;

use serenity::model::prelude::*;
use serenity::prelude::*;

impl Bot {
	pub async fn on_ready(&self, _ctx: Context, ready: Ready) -> Option<Infallible> {
		logger::debug("Getting ready...");

		// TODO Randomize self

		logger::info_fmt!(
			"Am ready :). I am {}. I am in {} mode",
			ready.user.tag(),
			if self.data.read().await.beta {
				"beta"
			} else {
				"normal"
			},
		);

		None
	}
}
