use crate::bot::Bot;

// Called from crate::event_handler
impl Bot {
	pub async fn periodic(&self) {
		self.pin_lock.lock().await.cleanup().await;
	}
}
