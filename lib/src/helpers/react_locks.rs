use crate::prelude::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::{oneshot, RwLock};

struct PinTask {
	pub channel: oneshot::Sender<()>,
	pub handle: tokio::task::JoinHandle<()>,
}

pub struct ReactSafety {
	finished: AtomicBool,
	timers: RwLock<Vec<Option<PinTask>>>,
}

impl Default for ReactSafety {
	fn default() -> Self {
		ReactSafety {
			finished: AtomicBool::new(false),
			timers: RwLock::new(Vec::new()),
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
		msg: &Message,
		reaction: &Reaction,
		required: Option<usize>,
		timeout: Option<std::time::Duration>,
	) -> bool {
		// The only way to access this function is by locking HallSafety, so we're, well, safe

		if self.finished.load(Ordering::Relaxed) {
			return false;
		}

		let Some(msg_reactions) = msg
			.reactions
			.iter()
			.find(|x| x.reaction_type == reaction.emoji)
		else {
			return false; // No reactions to speak of, cannot pin
		};

		if msg_reactions.me {
			return false; // No reactions if I've already reacted
		}

		let reactors = self
			.get_reactors(ctx, msg, reaction, required.unwrap_or(0))
			.await;

		if reactors.len() >= required.unwrap_or(0) {
			match msg.react(&ctx, reaction.emoji.clone()).await {
				Ok(reaction) => {
					if timeout.is_some() {
						let (send, recv) = oneshot::channel();
						let http = ctx.http.clone();
						let handle = tokio::spawn(async move {
							_ = tokio::time::timeout(timeout.unwrap(), recv).await;
							reaction.delete(&http).await.log_if_err(&format!(
								"Could not delete reaction {} from {}",
								reaction.emoji, reaction.message_id
							));
						});

						self.timers.write().await.push(Some(PinTask {
							channel: send,
							handle,
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
			false
		}
	}

	pub async fn cleanup(&mut self) {
		let mut lock = self.timers.write().await;

		let mut tail = lock.len();
		for i in 0..lock.len() {
			if lock[i].as_ref().is_some_and(|s| s.handle.is_finished()) {
				lock[i] = None;
				lock.swap(i, tail - 1);
				tail -= 1;
				if tail <= i {
					break;
				}
			}
		}
		lock.truncate(tail);
	}

	pub async fn terminate(&self) {
		self.finished.store(true, Ordering::Relaxed);

		let timers = std::mem::take(&mut *self.timers.write().await);

		futures::future::join_all(timers.into_iter().flatten().map(|s| {
			if !s.channel.is_closed() {
				_ = s.channel.send(()); // It is unimportant if the send is successful or not
			}
			s.handle
		}))
		.await;
	}
}
