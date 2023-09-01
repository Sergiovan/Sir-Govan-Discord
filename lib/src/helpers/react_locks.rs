use crate::prelude::*;
use serenity::http::CacheHttp;
use serenity::model::prelude::*;
use serenity::prelude::*;

use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::RwLock;

#[derive(thiserror::Error, Debug)]
pub enum ReactLockError {
	#[error("Incomplete custom emoji name: {0}")]
	CustomEmojiIncomplete(u64),
	#[error("Unable to get {1} reactions from {2}: {0}")]
	ReactionFetchError(#[source] serenity::Error, ReactionType, u64),
	#[error("Tried to pin-lock during cleanup")]
	CleanupInProgress,
	#[error("Unable to get msg {1} from {2}: {0}")]
	MessageFetchError(#[source] serenity::Error, u64, u64),
	#[error("Called react lock on message with no reactions")]
	NoReactions,
	#[error("")]
	AlreadyUsed,
	#[error("Unable to add {1} reaction to {2}: {0}")]
	ReactionAddError(#[source] serenity::Error, ReactionType, u64),
	#[error("")]
	NotEnoughReactors(usize, usize),
}

impl Reportable for ReactLockError {
	fn to_user(&self) -> Option<String> {
		match self {
			Self::CustomEmojiIncomplete(..) => {
				Some("It appears Discord bad. Try again later".to_string())
			}
			Self::MessageFetchError(..) | Self::ReactionFetchError(..) => {
				Some("My connection to the outside world is quite dodgy right now".to_string())
			}
			Self::ReactionAddError(..) => {
				Some("I can't react on that message, so I won't be touching it further".to_string())
			}
			_ => None,
		}
	}
}

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
	) -> Result<Vec<User>, ReactLockError> {
		// First get the emoji for sure
		if let ReactionType::Custom { name: None, id, .. } = reaction.emoji {
			return Err(ReactLockError::CustomEmojiIncomplete(id.into()));
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
						return Ok(res);
					}

					res.extend(filtered);
				}
				Err(e) => {
					return Err(ReactLockError::ReactionFetchError(
						e,
						reaction.emoji.clone(),
						msg.id.into(),
					));
				}
			};

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
	) -> Result<(), ReactLockError> {
		// The only way to access this function is by locking HallSafety, so we're, well, safe

		if self.bot_finished.load(Ordering::Relaxed) {
			return Err(ReactLockError::CleanupInProgress);
		}

		let msg = ctx
			.http
			.get_message(channel.into(), msg.0)
			.await
			.map_err(|e| ReactLockError::MessageFetchError(e, msg.into(), channel.into()))?;

		let msg_reactions = msg
			.reactions
			.iter()
			.find(|x| x.reaction_type == reaction.emoji)
			.ok_or(ReactLockError::NoReactions)?;

		if msg_reactions.me {
			return Err(ReactLockError::AlreadyUsed); // No reactions if I've already reacted
		}

		let reactors = self
			.get_reactors(ctx, &msg, reaction, required.unwrap_or(0))
			.await?;

		let required = required.unwrap_or(0);
		if reactors.len() >= required {
			let reaction = msg.react(&ctx, reaction.emoji.clone()).await.map_err(|e| {
				ReactLockError::ReactionAddError(e, reaction.emoji.clone(), msg.id.into())
			})?;

			if timeout.is_some() {
				self.tasks.write().await.push(Some(PinTask {
					start: std::time::Instant::now(),
					duration: timeout.unwrap(),
					reaction,
				}));
			}

			Ok(())
		} else {
			Err(ReactLockError::NotEnoughReactors(reactors.len(), required))
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
