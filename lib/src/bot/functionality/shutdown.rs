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
		self.pin_lock.lock().await.terminate().await;

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
