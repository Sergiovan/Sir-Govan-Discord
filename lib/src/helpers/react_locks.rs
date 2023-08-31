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
	) -> Vec<User> {
		// First get the emoji for sure
		if let ReactionType::Custom { name: None, .. } = reaction.emoji {
			logger::error_fmt!("Emoji from reaction was incomplete: {}", reaction.emoji);
			return vec![];
		};

		let mut last: Option<UserId> = None;
		let mut res = vec![];

		loop {
			// NOTE: Unknown how `last` interacts with the order in which reaction_users are returned
			match msg
				.reaction_users(&ctx, reaction.emoji.clone(), None, last)
				.await
			{
				Ok(users) => {
					let filtered = users
						.into_iter()
						.filter(|x| !x.bot && x.id != msg.author.id)
						.collect::<Vec<_>>();

					if filtered.is_empty() {
						return res;
					}

					res.extend(filtered);
				}
				Err(e) => {
					logger::error_fmt!(
						"Could not get {} reactions from {}: {}",
						reaction.emoji,
						msg.id,
						e
					);
					return res;
				}
			};

			if res.len() > required {
				return res;
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
	) -> bool {
		// The only way to access this function is by locking HallSafety, so we're, well, safe

		if self.bot_finished.load(Ordering::Relaxed) {
			logger::error("Tried to pin-lock while cleaning up");
			return false;
		}

		let msg = match ctx.http.get_message(channel.into(), msg.0).await {
			Ok(msg) => msg,
			Err(e) => {
				logger::error_fmt!("Fetching msg {} from {} did not work: {}", msg, channel, e);
				return false;
			}
		};

		let Some(msg_reactions) = msg
			.reactions
			.iter()
			.find(|x| x.reaction_type == reaction.emoji)
		else {
			logger::error("Called locked-react on message with no reactions");
			return false; // No reactions to speak of, cannot pin
		};

		if msg_reactions.me {
			logger::error("I already reacted to this");
			return false; // No reactions if I've already reacted
		}

		let reactors = self
			.get_reactors(ctx, &msg, reaction, required.unwrap_or(0))
			.await;

		if reactors.len() >= required.unwrap_or(0) {
			match msg.react(&ctx, reaction.emoji.clone()).await {
				Ok(reaction) => {
					if timeout.is_some() {
						self.tasks.write().await.push(Some(PinTask {
							start: std::time::Instant::now(),
							duration: timeout.unwrap(),
							reaction,
						}));
					}
					true
				}
				Err(e) => {
					logger::error_fmt!(
						"Error while adding {} reaction to {}: {}",
						reaction.emoji,
						msg.id,
						e
					);
					false
				}
			}
		} else {
			logger::error_fmt!("Not enough reactors: {} < {:?}", reactors.len(), required);
			false
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

		futures::future::join_all(vec.into_iter()).await;
	}

	pub async fn terminate(&self, ctx: &impl CacheHttp) {
		self.bot_finished.store(true, Ordering::Relaxed);

		let timers = std::mem::take(&mut *self.tasks.write().await);

		futures::future::join_all(timers.into_iter().flatten().map(|t| t.resolve(ctx))).await;
	}
}
