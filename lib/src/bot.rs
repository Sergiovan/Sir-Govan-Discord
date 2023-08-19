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
	commander: RwLock<Commander>,
	pin_lock: Mutex<ReactSafety>,
	shard_manager: RwLock<Option<Arc<Mutex<ShardManager>>>>,
	shutdown: Mutex<bool>,
	screenshotter: RwLock<Option<Screenshotter>>,
}

impl Bot {
	pub fn new(data: BotData) -> Bot {
		Bot {
			data: RwLock::new(data),
			commander: RwLock::new(Commander::new()),
			pin_lock: Mutex::new(ReactSafety::default()),
			shard_manager: RwLock::new(None),
			shutdown: Mutex::new(false),
			screenshotter: RwLock::new(None),
		}
	}

	pub async fn set_shard_manager(&self, shard_manager: Arc<Mutex<ShardManager>>) {
		*self.shard_manager.write().await = Some(shard_manager)
	}

	pub async fn init_screenshotter(&self) -> bool {
		let screenshotter = Screenshotter::new().ok_or_log("Could not load screenshotter data");

		let res = screenshotter.is_some();

		*self.screenshotter.write().await = screenshotter;

		res
	}
}
