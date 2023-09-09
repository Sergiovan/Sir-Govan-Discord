use crate::bot::Bot;
use crate::util::logger;

use async_trait::async_trait;
use serenity::json::Value;
use serenity::model::prelude::*;
use serenity::prelude::*;

use std::sync::Arc;

pub struct BotEventHandler {
	bot: Arc<Bot>,
}

impl BotEventHandler {
	pub fn new(bot: Arc<Bot>) -> BotEventHandler {
		BotEventHandler { bot }
	}
}

#[async_trait]
impl EventHandler for BotEventHandler {
	async fn ready(&self, ctx: Context, ready: Ready) {
		self.bot.on_ready(ctx, ready).await;
	}

	async fn resume(&self, _ctx: Context, _: ResumedEvent) {
		// TODO
		logger::debug("Reconnected :)");
	}

	async fn unknown(&self, _ctx: Context, name: String, raw: Value) {
		// TODO
		logger::debug_fmt!("Unknown event {} occurred: {}", name, raw);
	}

	async fn message(&self, ctx: Context, msg: Message) {
		if let Err(e) = self.bot.on_message(&ctx, &msg).await {
			e.report(
				&ctx,
				either::Either::Left(&msg),
				Some(&self.bot.data.read().await.strings),
			)
			.await;
		}
	}

	async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
		if let Err(e) = self.bot.on_reaction_add(&ctx, &add_reaction).await {
			match add_reaction.message(&ctx).await {
				Ok(msg) => {
					e.report(
						&ctx,
						either::Left(&msg),
						Some(&self.bot.data.read().await.strings),
					)
					.await;
				}
				Err(err) => {
					e.log();
					logger::error_fmt!(
						"While reporting last error another error occurred: {}",
						err
					);
				}
			}
		}
	}
}
