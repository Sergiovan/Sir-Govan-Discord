pub mod fake_iasip;
pub mod fake_twitter;
pub mod halls;
pub mod no_context;
pub mod periodic;
pub mod randomize_self;
pub mod shutdown;

use serenity::client::bridge::gateway::ShardManager;
use serenity::{prelude::*, CacheAndHttp};

use self::periodic::Periodic;

use super::commands::commander::Commander;
use super::data::BotData;
use super::helpers::react_locks::ReactSafety;
use super::helpers::screenshotter::Screenshotter;
use crate::util::traits::ResultExt;
use std::sync::Arc;

pub struct Bot {
	pub(crate) data: RwLock<BotData>,
	pub(crate) commander: Mutex<Commander>,
	pub(crate) pin_lock: Mutex<ReactSafety>,
	pub(crate) shard_manager: RwLock<Option<Arc<Mutex<ShardManager>>>>,
	pub(crate) cache_and_http: RwLock<Option<Arc<CacheAndHttp>>>,
	pub(crate) shutdown: Mutex<bool>,
	pub(crate) screenshotter: RwLock<Option<Screenshotter>>,
	pub(crate) periodic: Mutex<Periodic>,
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
			cache_and_http: RwLock::new(None),
			shutdown: Mutex::new(false),
			screenshotter: RwLock::new(None),
			periodic: Mutex::new(Periodic::new()),
		}
	}

	pub async fn set_shard_manager(&self, shard_manager: Arc<Mutex<ShardManager>>) {
		*self.shard_manager.write().await = Some(shard_manager)
	}

	pub async fn set_cache_and_http(&self, http: Arc<CacheAndHttp>) {
		*self.cache_and_http.write().await = Some(http);
	}

	pub async fn get_screenshotter(&self) -> tokio::sync::RwLockReadGuard<Option<Screenshotter>> {
		{
			let lock = self.screenshotter.read().await;
			if lock.is_some() {
				return lock;
			}
		}

		let screenshotter = Screenshotter::new()
			.await
			.ok_or_log("Could not load screenshotter data");

		*self.screenshotter.write().await = screenshotter;

		self.screenshotter.read().await
	}
}
