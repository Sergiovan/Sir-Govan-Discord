use serenity::client::bridge::gateway::ShardManager;
use serenity::prelude::*;

use super::commands::commander::Commander;
use super::data::BotData;
use super::helpers::react_locks::ReactSafety;
use super::helpers::screenshotter::Screenshotter;
use super::util::traits::ResultExt;
use std::sync::Arc;

pub struct Bot {
	pub(crate) data: RwLock<BotData>,
	pub(crate) commander: Mutex<Commander>,
	pub(crate) pin_lock: Mutex<ReactSafety>,
	pub(crate) shard_manager: RwLock<Option<Arc<Mutex<ShardManager>>>>,
	pub(crate) shutdown: Mutex<bool>,
	pub(crate) screenshotter: RwLock<Option<Screenshotter>>,
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
