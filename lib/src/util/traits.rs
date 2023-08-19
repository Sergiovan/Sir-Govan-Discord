use super::logger;
use async_trait::async_trait;

use serenity::model::prelude::*;
use serenity::prelude::*;

use regex::Regex;
use thiserror::Error;

pub trait ResultExt<T> {
	fn log_if_err(self, msg: &str);
	fn ok_or_log(self, msg: &str) -> Option<T>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
	fn log_if_err(self, msg: &str) {
		match self {
			Ok(_) => (),
			Err(e) => {
				logger::error(&format!("{}: {}", msg, e));
			}
		}
	}

	fn ok_or_log(self, msg: &str) -> Option<T> {
		match self {
			Ok(t) => Some(t),
			Err(e) => {
				logger::error(&format!("{}: {}", msg, e));
				None
			}
		}
	}
}

pub trait OptionExt<T> {
	fn log_if_none(self, msg: &str) -> Self;
}

impl<T> OptionExt<T> for Option<T> {
	fn log_if_none(self, msg: &str) -> Self {
		match self {
			Some(_) => (),
			None => logger::error(msg),
		}
		self
	}
}

#[async_trait]
pub trait CacheGuild {
	async fn guild_cached(&self, ctx: &Context) -> bool;
}

#[async_trait]
impl CacheGuild for Message {
	async fn guild_cached(&self, ctx: &Context) -> bool {
		if self.guild_id.is_some() && self.guild(ctx).is_none() {
			if let Err(e) = ctx
				.http
				.get_guild(
					*self
						.guild_id
						.expect("Guild somehow disappeared in between lines")
						.as_u64(),
				)
				.await
			{
				logger::error(&format!(
					"Could not get guild information for {} from message {}: {}",
					self.guild_id.unwrap(),
					self.id,
					e
				));
				return false;
			}
		}

		true
	}
}

#[async_trait]
impl CacheGuild for GuildChannel {
	async fn guild_cached(&self, ctx: &Context) -> bool {
		if self.guild(ctx).is_none() {
			if let Err(e) = ctx.http.get_guild(*self.guild_id.as_u64()).await {
				logger::error(&format!(
					"Could not get guild information for {} from channel {}: {}",
					self.id, self.guild_id, e
				));
				return false;
			}
		}

		true
	}
}

#[derive(Debug, Error)]
pub enum UniqueColorError {
	#[error("could not find guild")]
	GuildMissing,
	#[error("could not get roles")]
	RolesMissing,
	#[error("member does not have any colored roles")]
	NoColoredRole,
}

pub trait MemberExt {
	fn get_unique_color(&self, ctx: &Context) -> Result<Role, UniqueColorError>;
}

impl MemberExt for Member {
	fn get_unique_color(&self, ctx: &Context) -> Result<Role, UniqueColorError> {
		use serenity::utils::Colour;

		let guild = match ctx.cache.guild(self.guild_id) {
			Some(g) => g,
			None => return Err(UniqueColorError::GuildMissing),
		};

		let mut roles = match self.roles(ctx) {
			Some(r) => r,
			None => return Err(UniqueColorError::RolesMissing),
		};

		roles.sort_by_key(|r| r.position);

		for role in roles.iter().rev() {
			if role.colour == Colour(0) {
				continue;
			}

			let other = guild
				.members
				.iter()
				.any(|(id, m)| id != &self.user.id && m.roles.contains(&role.id));
			if !other {
				return Ok(role.clone());
			}
		}

		Err(UniqueColorError::NoColoredRole)
	}
}

pub trait MatchMap {
	fn match_map<'a, F, T>(
		&'a self,
		regex: &Regex,
		f: F,
	) -> std::iter::Map<std::vec::IntoIter<(&'a str, bool)>, F>
	where
		F: FnMut((&'a str, bool)) -> T;
}

impl<S: AsRef<str>> MatchMap for S {
	fn match_map<'a, F, T>(
		&'a self,
		regex: &Regex,
		f: F,
	) -> std::iter::Map<std::vec::IntoIter<(&'a str, bool)>, F>
	where
		F: FnMut((&'a str, bool)) -> T,
	{
		let this = self.as_ref();
		let mut pieces: Vec<(&str, bool)> = vec![];

		let mut current = 0_usize;
		for regex_match in regex.find_iter(this) {
			let start = regex_match.start();
			let end = regex_match.end();

			if start != current {
				pieces.push((&this[current..start], false));
			}
			current = end;
			pieces.push((&this[regex_match.range()], true));
		}

		if current != this.len() {
			pieces.push((&this[current..], false));
		}

		pieces.into_iter().map(f)
	}
}
