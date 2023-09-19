use crate::prelude::*;
use serenity::http::CacheHttp;
use serenity::model::prelude::*;
use serenity::prelude::*;

use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::RwLock;

struct PinTask {
	pub start: std::time::Instant,
	pub duration: std::time::Duration,
	pub reaction: Reaction,
}

impl PinTask {
	pub async fn resolve(self, ctx: &impl CacheHttp) {
		self.reaction.delete(&ctx).await.log_if_err(&format!(
			"Could not delete reaction {} from {}",
			self.reaction.emoji, self.reaction.message_id
		));
	}

	pub fn finished(&self) -> bool {
		self.start + self.duration < std::time::Instant::now()
	}
}

pub struct ReactSafety {
	bot_finished: AtomicBool,
	tasks: RwLock<Vec<Option<PinTask>>>,
}

impl Default for ReactSafety {
	fn default() -> Self {
		ReactSafety {
			bot_finished: AtomicBool::new(false),
			tasks: RwLock::new(Vec::new()),
		}
	}
}

impl ReactSafety {
	async fn get_reactors(
		&self,
		ctx: &Context,
		msg: &Message,
		reaction: &Reaction,
		required: usize,
	) -> GovanResult<Vec<User>> {
		// First get the emoji for sure
		if let ReactionType::Custom { name: None, id, .. } = reaction.emoji {
			return Err(govanerror::error!(
			  log fmt = ("Incomplete custom emoji name: {}", id),
			  user = "It appears Discord bad. Try again later"
			));
		};

		let mut last: Option<UserId> = None;
		let mut res = vec![];

		loop {
			// NOTE: Unknown how `last` interacts with the order in which reaction_users are returned
			let users = msg
				.reaction_users(&ctx, reaction.emoji.clone(), None, last)
				.await?;

			let filtered = users
				.into_iter()
				.filter(|x| !x.bot && x.id != msg.author.id)
				.collect::<Vec<_>>();

			if filtered.is_empty() {
				return Ok(res);
			}

			res.extend(filtered);

			if res.len() > required {
				return Ok(res);
			}

			last = res.last().map(|x| x.id);
		}
	}

	pub async fn locked_react(
		&self,
		ctx: &Context,
		msg: MessageId,
		channel: ChannelId,
		reaction: &Reaction,
		required: Option<usize>,
		timeout: Option<std::time::Duration>,
	) -> GovanResult {
		// The only way to access this function is by locking HallSafety, so we're, well, safe

		if self.bot_finished.load(Ordering::Relaxed) {
			return Err(govanerror::error!(
				log = "Trying to react while bot shutting down"
			));
		}

		let msg = ctx.http.get_message(channel, msg).await?;

		let msg_reactions = msg
			.reactions
			.iter()
			.find(|x| x.reaction_type == reaction.emoji)
			.ok_or_else(govanerror::warning_lazy!(log fmt = ("No reactions of type {} on message {}", reaction.emoji, msg.id)))?;

		if msg_reactions.me {
			return Err(govanerror::debug!(log = "Already reacted")); // No reactions if I've already reacted
		}

		let reactors = self
			.get_reactors(ctx, &msg, reaction, required.unwrap_or(0))
			.await?;

		let required = required.unwrap_or(0);
		if reactors.len() >= required {
			let reaction = msg.react(&ctx, reaction.emoji.clone()).await?;

			if timeout.is_some() {
				self.tasks.write().await.push(Some(PinTask {
					start: std::time::Instant::now(),
					duration: timeout.unwrap(),
					reaction,
				}));
			}

			Ok(())
		} else {
			Err(govanerror::debug!(
				log fmt = ("Not enough reactions: {} < {}", reactors.len(), required)
			))
		}
	}

	pub async fn cleanup(&mut self, ctx: &impl CacheHttp) {
		let mut lock = self.tasks.write().await;

		let mut vec = Vec::new();

		lock.retain_mut(|t| {
			if t.as_ref().is_some_and(|t| t.finished()) {
				let t = t.take().unwrap();
				vec.push(t.resolve(ctx));
				false
			} else {
				t.is_some()
			}
		});

		util::collect_async(vec.into_iter()).await;
	}

	pub async fn terminate(&self, http: &impl CacheHttp) {
		self.bot_finished.store(true, Ordering::Relaxed);

		let timers = std::mem::take(&mut *self.tasks.write().await);

		util::collect_async(timers.into_iter().flatten().map(|t| t.resolve(&http))).await;
	}
}
