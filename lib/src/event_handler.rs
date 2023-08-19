use crate::bot::Bot;
use crate::util::logger;

use serenity::async_trait;
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
		{
			let bot = self.bot.clone();
			tokio::spawn(async move {
				loop {
					_ = tokio::time::timeout(tokio::time::Duration::from_secs(300), bot.periodic())
						.await;
				}
			});
		}

		self.bot.on_ready(ctx, ready).await;
	}

	async fn resume(&self, _ctx: Context, _: ResumedEvent) {
		// TODO
		logger::debug("Reconnected :)");
	}

	async fn unknown(&self, _ctx: Context, _name: String, _raw: Value) {
		// TODO
		logger::debug("wtf");
	}

	async fn message(&self, ctx: Context, msg: Message) {
		self.bot.on_message(ctx, msg).await;
	}

	async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
		self.bot.on_reaction_add(ctx, add_reaction).await;
	}
}
