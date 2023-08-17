mod commands;
mod functionality;
mod handlers;

pub mod data;
use serenity::client::bridge::gateway::ShardManager;
use serenity::prelude::*;

use self::commands::commander::Commander;
use self::data::BotData;
use self::functionality::react_locks::ReactSafety;
use std::sync::Arc;

pub struct Bot {
	data: RwLock<BotData>,
	commander: RwLock<Commander>,
	pin_lock: Mutex<ReactSafety>,
	shard_manager: RwLock<Option<Arc<Mutex<ShardManager>>>>,
	shutdown: Mutex<bool>,
}

impl Bot {
	pub fn new(data: BotData) -> Bot {
		Bot {
			data: RwLock::new(data),
			commander: RwLock::new(Commander::new()),
			pin_lock: Mutex::new(ReactSafety::default()),
			shard_manager: RwLock::new(None),
			shutdown: Mutex::new(false),
		}
	}

	pub async fn set_shard_manager(&self, shard_manager: Arc<Mutex<ShardManager>>) {
		*self.shard_manager.write().await = Some(shard_manager)
	}
}
