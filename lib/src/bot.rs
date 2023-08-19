mod commands;
mod functionality;
mod handlers;
mod helpers;

pub mod data;
use serenity::client::bridge::gateway::ShardManager;
use serenity::prelude::*;

use self::commands::commander::Commander;
use self::data::BotData;
use self::helpers::react_locks::ReactSafety;
use self::helpers::screenshotter::Screenshotter;
use super::util::ResultErrorHandler;
use std::sync::Arc;

pub struct Bot {
	data: RwLock<BotData>,
	commander: Mutex<Commander>,
	pin_lock: Mutex<ReactSafety>,
	shard_manager: RwLock<Option<Arc<Mutex<ShardManager>>>>,
	shutdown: Mutex<bool>,
	screenshotter: RwLock<Option<Screenshotter>>,
}

impl Bot {
	pub fn new(data: BotData) -> Bot {
		let mut commander = Commander::new();
		commander.register_all();

		Bot {
			data: RwLock::new(data),
			commander: Mutex::new(commander),
			pin_lock: Mutex::new(ReactSafety::default()),
			shard_manager: RwLock::new(None),
			shutdown: Mutex::new(false),
			screenshotter: RwLock::new(None),
		}
	}

	pub async fn set_shard_manager(&self, shard_manager: Arc<Mutex<ShardManager>>) {
		*self.shard_manager.write().await = Some(shard_manager)
	}

	pub async fn get_screenshotter(&self) -> tokio::sync::RwLockReadGuard<Option<Screenshotter>> {
		{
			let lock = self.screenshotter.read().await;
			if lock.is_some() {
				return lock;
			}
		}

		let screenshotter = Screenshotter::new().ok_or_log("Could not load screenshotter data");

		*self.screenshotter.write().await = screenshotter;

		self.screenshotter.read().await
	}
}
