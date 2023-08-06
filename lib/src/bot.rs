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
use self::data::BotData;
use self::functionality::halls::HallSafety;
use std::sync::Arc;

pub struct Bot {
	pub data: Arc<RwLock<BotData>>,
	pub commander: Arc<RwLock<Commander>>,
	pub pin_lock: Arc<Mutex<HallSafety>>,
}

impl Bot {
	pub fn new(data: BotData) -> Bot {
		Bot {
			data: Arc::new(RwLock::new(data)),
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
