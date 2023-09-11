use super::Bot;
use crate::prelude::*;

use serenity::gateway::ActivityData;
use serenity::model::prelude::*;

use crate::util::random;

impl Bot {
	pub async fn randomize_self(&self) {
		logger::info("Randomizing self...");

		let bot_data = &self.data().await;

		let http = self.http().await;
		let cache = self.cache().await;

		for (&id, server) in bot_data.servers.iter() {
			let guild = match GuildId::new(id).to_partial_guild(&http).await {
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
				.edit_nickname(&http, nickname.map(String::as_str))
				.await
			{
				logger::error_fmt!("{}", e);
			}

			logger::debug_fmt!(
				"Set name to {} in {}",
				nickname.unwrap_or(&cache.current_user().name),
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
			Some(ActivityType::Playing) => Some(ActivityData::playing(
				bot_data.strings.status_playing.pick(),
			)),
			Some(ActivityType::Listening) => Some(ActivityData::listening(
				bot_data.strings.status_listening.pick(),
			)),
			Some(ActivityType::Watching) => Some(ActivityData::watching(
				bot_data.strings.status_watching.pick(),
			)),
			Some(ActivityType::Streaming) => {
				let strings = &bot_data.strings;
				let default = String::from("Secrets upon secrets");

				ActivityData::streaming(
					random::pick_or(
						&vec![
							strings.status_playing.pick(),
							strings.status_listening.pick(),
							strings.status_watching.pick(),
						],
						&&default,
					)
					.to_string(),
					"...",
				)
				.ok()
				.or(None)
			}
			Some(ActivityType::Competing) => None,
			_ => None,
		};

		let shard_manager = self.shard_manager().await;

		for (.., runner) in shard_manager.runners.lock().await.iter() {
			runner.runner_tx.set_activity(activity.clone());
		}

		logger::debug_fmt!("Set activity to {:?}", activity);
	}
}
