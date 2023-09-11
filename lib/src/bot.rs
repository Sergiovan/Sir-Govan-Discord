pub mod fake_iasip;
pub mod fake_twitter;
pub mod halls;
pub mod no_context;
pub mod periodic;
pub mod randomize_self;
pub mod shutdown;

use serenity::client::Cache;
use serenity::gateway::ShardManager;
use serenity::http::Http;
use serenity::prelude::*;

use self::periodic::Periodic;

use super::commands::commander::Commander;
use super::data::BotData;
use super::helpers::react_locks::ReactSafety;
use super::helpers::screenshotter::Screenshotter;
use crate::prelude::{govanerror, GovanResult};
use std::sync::Arc;

pub struct Bot {
	pub(crate) data: RwLock<BotData>,
	pub(crate) commander: Commander,
	pub(crate) pin_lock: Mutex<ReactSafety>,
	pub(crate) shard_manager: RwLock<Option<Arc<ShardManager>>>,
	pub(crate) cache: RwLock<Option<Arc<Cache>>>,
	pub(crate) http: RwLock<Option<Arc<Http>>>,
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
			commander,
			pin_lock: Mutex::new(ReactSafety::default()),
			shard_manager: RwLock::new(None),
			cache: RwLock::new(None),
			http: RwLock::new(None),
			shutdown: Mutex::new(false),
			screenshotter: RwLock::new(None),
			periodic: Mutex::new(Periodic::new()),
		}
	}

	pub async fn data(&self) -> tokio::sync::RwLockReadGuard<BotData> {
		self.data.read().await
	}

	pub async fn pin_lock(&self) -> tokio::sync::MutexGuard<ReactSafety> {
		self.pin_lock.lock().await
	}

	pub async fn set_shard_manager(&self, shard_manager: Arc<ShardManager>) {
		*self.shard_manager.write().await = Some(shard_manager)
	}

	pub async fn shard_manager(&self) -> Arc<ShardManager> {
		self.shard_manager
			.read()
			.await
			.clone()
			.expect("No shard manager set yet")
	}

	pub async fn set_cache_and_http(&self, cache: Arc<Cache>, http: Arc<Http>) {
		*self.cache.write().await = Some(cache);
		*self.http.write().await = Some(http);
	}

	pub async fn cache(&self) -> Arc<Cache> {
		self.cache.read().await.clone().expect("No cache set yet")
	}

	pub async fn http(&self) -> Arc<Http> {
		self.http.read().await.clone().expect("No cache set yet")
	}

	pub async fn set_screenshotter(&self) -> GovanResult {
		let screenshotter = Screenshotter::new().await?;

		*self.screenshotter.write().await = Some(screenshotter);

		Ok(())
	}

	pub async fn screenshotter(&self) -> GovanResult<tokio::sync::RwLockReadGuard<Screenshotter>> {
		let lock = self.screenshotter.read().await;

		if lock.is_none() {
			return Err(govanerror::error!(
				log = "No screenshotter set",
				user = "My camera broke :("
			));
		}

		Ok(tokio::sync::RwLockReadGuard::map(lock, |o| {
			o.as_ref().expect("No screenshotter set yet")
		}))
	}

	pub async fn periodic(&self) -> tokio::sync::MutexGuard<Periodic> {
		self.periodic.lock().await
	}
}
