use crate::bot::Bot;
use crate::util::logger;

impl Bot {
	pub async fn shutdown(&self) {
		let mut shutdown = self.shutdown.lock().await;

		if *shutdown {
			return;
		}

		logger::info("Shutting down...");
		// Other shutdown code

		self.periodic.lock().await.end_periodic().await;

		match &*self.cache_and_http.read().await {
			Some(cache_and_http) => self.pin_lock.lock().await.terminate(cache_and_http).await,
			None => (),
		}

		match &*self.shard_manager.read().await {
			None => {
				logger::error("Called shutdown before taking control of shard manager");
			}
			Some(l) => {
				l.lock().await.shutdown_all().await;
			}
		};

		logger::info("Bye!");
		*shutdown = true;
	}
}
