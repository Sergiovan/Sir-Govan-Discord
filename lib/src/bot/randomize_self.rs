use super::Bot;
use crate::prelude::*;

use serenity::http::CacheHttp;
use serenity::model::prelude::*;

use crate::util::random;

impl Bot {
	pub async fn randomize_self(&self) {
		logger::info("Randomizing self...");

		let bot_data = &self.data.read().await;

		let ctx = self.cache_and_http.read().await;
		let ctx = ctx.as_ref().unwrap();

		for (&id, server) in bot_data.servers.iter() {
			let guild = match GuildId(id).to_partial_guild(&ctx.http()).await {
				Ok(guild) => guild,
				Err(e) => {
					logger::error_fmt!("Couldn't find {}: {}", id, e);
					continue;
				}
			};

			let nickname = bot_data
				.strings
				.nickname
				.pick()
				.or(server.nickname.as_ref());

			if let Err(e) = guild
				.edit_nickname(ctx.http(), nickname.map(String::as_str))
				.await
			{
				logger::error_fmt!("{}", e);
			}

			logger::debug_fmt!(
				"Set name to {} in {}",
				nickname.unwrap_or(&ctx.cache.current_user().name),
				guild.name
			)
		}

		let activity_bag = random::GrabBagBuilder::new()
			.rare(random::GrabBagTier::maybe_rare(Some(vec![
				ActivityType::Playing,
				ActivityType::Listening,
				ActivityType::Watching,
			])))
			.mythical(random::GrabBagTier::maybe_mythical(Some(vec![
				ActivityType::Streaming,
				ActivityType::Competing,
			])))
			.finish_loose(None);

		if let Err(e) = activity_bag {
			logger::error_fmt!("{}", e);
			return;
		}

		let activity_bag = activity_bag.unwrap();

		let activity = match activity_bag.pick() {
			None => None,
			Some(ActivityType::Playing) => {
				Some(Activity::playing(bot_data.strings.status_playing.pick()))
			}
			Some(ActivityType::Listening) => Some(Activity::listening(
				bot_data.strings.status_listening.pick(),
			)),
			Some(ActivityType::Watching) => {
				Some(Activity::watching(bot_data.strings.status_watching.pick()))
			}
			Some(ActivityType::Streaming) => {
				let strings = &bot_data.strings;
				let default = String::from("Secrets upon secrets");
				Some(Activity::streaming(
					random::pick_or(
						&vec![
							strings.status_playing.pick(),
							strings.status_listening.pick(),
							strings.status_watching.pick(),
						],
						&&default,
					),
					"...",
				))
			}
			Some(ActivityType::Competing) => None,
			_ => None,
		};

		let shard_manager = self.shard_manager.read().await;
		let shard_manager = shard_manager.as_ref().unwrap();

		for (.., runner) in shard_manager.lock().await.runners.lock().await.iter() {
			runner.runner_tx.set_activity(activity.clone());
		}

		logger::debug_fmt!("Set activity to {:?}", activity);
	}
}
