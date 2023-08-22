use std::convert::Infallible;

use crate::bot::Bot;
use crate::prelude::ResultExt;
use crate::util::logger;

use serenity::model::prelude::*;
use serenity::prelude::*;

impl Bot {
	pub async fn on_ready(&self, ctx: Context, ready: Ready) -> Option<Infallible> {
		logger::debug("Getting ready...");

		// TODO Randomize self

		logger::info(&format!(
			"Am ready :). I am {}. I am in {} mode",
			ready.user.tag(),
			if self.data.read().await.beta {
				"beta"
			} else {
				"normal"
			}
		));

		None
	}
}
