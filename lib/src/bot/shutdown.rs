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

		self.periodic().await.end_periodic().await;

		self.pin_lock()
			.await
			.terminate(&self.cache_and_http().await)
			.await;

		self.shard_manager().await.lock().await.shutdown_all().await;

		logger::info("Bye!");
		*shutdown = true;
	}
}
