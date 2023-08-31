use crate::util::logger;

use crate::bot::Bot;

use std::sync::Arc;

pub struct Periodic {
	handle: Option<tokio::task::JoinHandle<()>>,
	handle_ender: Option<tokio::sync::mpsc::Sender<()>>,
}

impl Periodic {
	pub fn new() -> Periodic {
		Periodic {
			handle: None,
			handle_ender: None,
		}
	}

	pub fn spawn_periodic(&mut self, bot: Arc<Bot>) {
		if self.handle.is_some() {
			return;
		}

		let (send, mut recv) = tokio::sync::mpsc::channel(1);

		self.handle_ender = Some(send);

		self.handle = Some(tokio::spawn(async move {
			loop {
				let res =
					tokio::time::timeout(tokio::time::Duration::from_secs(30), recv.recv()).await;

				bot.periodic().await;

				if res.is_ok() {
					recv.close();
					return;
				}
			}
		}));
	}

	pub async fn end_periodic(&mut self) {
		if self.handle_ender.is_none() != self.handle.is_none() {
			logger::error_fmt!("Period handle and sender channel are in an invalid state: Sender {:?} and Handle {:?}", self.handle_ender, self.handle);
			panic!();
		}

		if self.handle_ender.is_none() && self.handle.is_none() {
			return; // Nothing to do
		}

		let sender = self.handle_ender.take().unwrap();
		let handle = self.handle.take().unwrap();

		_ = sender.send(()).await;
		_ = handle.await;
	}
}

// Called from crate::event_handler
impl Bot {
	pub async fn periodic(&self) {
		self.pin_lock
			.lock()
			.await
			.cleanup(self.cache_and_http.read().await.as_ref().unwrap())
			.await;
	}
}
