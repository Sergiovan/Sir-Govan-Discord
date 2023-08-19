use std::convert::Infallible;

use crate::bot::Bot;
use crate::util::logger;

use serenity::model::prelude::*;
use serenity::prelude::*;

impl Bot {
	pub async fn on_ready(&self, _ctx: Context, ready: Ready) -> Option<Infallible> {
		logger::debug("Getting ready...");

		self.commander.write().await.register_all();

		let bot_data = &self.data;

		use crate::bot::data::config;
		let data = match config::read_servers() {
			Ok(data) => data,
			Err(config::ServerTomlError::IO(err)) => {
				logger::error(&format!("Could not open the settings file: {}", err));
				self.shutdown().await;
				return None;
			}
			Err(config::ServerTomlError::Toml(err)) => {
				logger::error(&format!("Could not parse the settings file: {}", err));
				self.shutdown().await;
				return None;
			}
		};

		let is_beta = bot_data.read().await.beta;

		{
			let mut bot_data = bot_data.write().await;
			bot_data.servers.extend(
				data
					.servers
					.into_iter()
					.filter(|server| server.beta == is_beta)
					.map(|server| (server.id, server.into())),
			);
			match bot_data.load_no_context() {
				Ok(()) => (),
				Err(e) => {
					logger::error(&format!("Could not load no context roles from file: {}", e));
					self.shutdown().await;
					return None;
				}
			}
		}

		logger::info(&format!(
			"Am ready :). I am {}. I am in {} mode",
			ready.user.tag(),
			if bot_data.read().await.beta {
				"beta"
			} else {
				"normal"
			}
		));

		None
	}
}
