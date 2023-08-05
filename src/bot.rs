mod commands;
mod functionality;
mod handlers;

pub mod data;

use serenity::async_trait;
use serenity::json::Value;
use serenity::model::prelude::*;
use serenity::prelude::*;

use serenity::model::gateway::Ready;
use tracing::debug;

use self::commands::commander::Commander;
use self::functionality::halls::HallSafety;
use std::sync::Arc;

use crate::util::logger;

pub struct Bot {
	pub commander: Arc<RwLock<Commander>>,
	pub pin_lock: Arc<Mutex<HallSafety>>,
}

impl Default for Bot {
	fn default() -> Self {
		Bot {
			commander: Arc::new(RwLock::new(Commander::new())),
			pin_lock: Arc::new(Mutex::new(HallSafety)),
		}
	}
}

#[async_trait]
impl EventHandler for Bot {
	async fn ready(&self, ctx: Context, ready: Ready) {
		self.on_ready(ctx, ready).await;
	}

	async fn resume(&self, _ctx: Context, _: ResumedEvent) {
		// TODO
		debug!("Reconnected :)");
	}

	async fn unknown(&self, _ctx: Context, _name: String, _raw: Value) {
		// TODO
		debug!("wtf");
	}

	async fn message(&self, ctx: Context, msg: Message) {
		self.on_message(ctx, msg).await;
	}

	async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
		self.on_reaction_add(ctx, add_reaction).await;
	}
}

#[async_trait]
trait CacheGuild {
	async fn guild_cached(&self, ctx: &Context) -> bool;
}

#[async_trait]
impl CacheGuild for Message {
	async fn guild_cached(&self, ctx: &Context) -> bool {
		if self.guild_id.is_some() && self.guild(ctx).is_none() {
			if let Err(e) = ctx
				.http
				.get_guild(
					*self
						.guild_id
						.expect("Guild somehow disappeared in between lines")
						.as_u64(),
				)
				.await
			{
				logger::error(&format!(
					"Could not get guild information for {} from message {}: {}",
					self.guild_id.unwrap(),
					self.id,
					e
				));
				return false;
			}
		}

		true
	}
}

#[async_trait]
impl CacheGuild for GuildChannel {
	async fn guild_cached(&self, ctx: &Context) -> bool {
		if self.guild(ctx).is_none() {
			if let Err(e) = ctx.http.get_guild(*self.guild_id.as_u64()).await {
				logger::error(&format!(
					"Could not get guild information for {} from channel {}: {}",
					self.id, self.guild_id, e
				));
				return false;
			}
		}

		true
	}
}
