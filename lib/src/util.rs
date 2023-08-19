pub mod logger;
pub mod random;

use lazy_static::lazy_static;
use regex::Regex;
use serenity::model::prelude::*;
use serenity::{async_trait, prelude::*};

use serenity::utils::Colour;

// Static values
lazy_static! {
	pub static ref EMOJI_REGEX: Regex = Regex::new(r"\p{RI}\p{RI}|[\p{Emoji}--\p{Ascii}](?:\p{EMod}|\x{FE0F}\x{20E3}?|[\x{E0020}-\x{E007E}]+\x{E007F})?(?:\x{200D}\p{Emoji}(?:\p{EMod}|\x{FE0F}\x{20E3}?|[\x{E0020}-\x{E007E}]+\x{E007F})?)*|[0-9]\x{FE0F}\x{20e3}",).unwrap();
	pub static ref DISCORD_EMOJI_REGEX: Regex = Regex::new(r"<(a?):(.*?):([0-9]+)>").unwrap();
}

// Enums
pub enum UniqueColorError {
	GuildMissing,
	RolesMissing,
	NoColoredRole,
}

// Traits and Impls
pub trait ResultErrorHandler<T> {
	fn log_if_err(self, msg: &str);
	fn ok_or_log(self, msg: &str) -> Option<T>;
}

impl<T, E: std::fmt::Display> ResultErrorHandler<T> for Result<T, E> {
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

pub trait OptionErrorHandler<T> {
	fn log_if_none(self, msg: &str) -> Self;
}

impl<T> OptionErrorHandler<T> for Option<T> {
	fn log_if_none(self, msg: &str) -> Self {
		match self {
			Some(_) => (),
			None => logger::error(msg),
		}
		self
	}
}

pub trait NickOrName {
	fn get_name(&self) -> &str;
}

impl NickOrName for Member {
	fn get_name(&self) -> &str {
		self.nick.as_ref().unwrap_or(&self.user.name)
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

// Free functions

pub fn get_unique_color(ctx: &Context, member: &Member) -> Result<Role, UniqueColorError> {
	let guild = match ctx.cache.guild(member.guild_id) {
		Some(g) => g,
		None => return Err(UniqueColorError::GuildMissing),
	};

	let mut roles = match member.roles(ctx) {
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
			.any(|(id, m)| id != &member.user.id && m.roles.contains(&role.id));
		if !other {
			return Ok(role.clone());
		}
	}

	Err(UniqueColorError::NoColoredRole)
}

pub fn filename_from_unicode_emoji(emoji: &str) -> String {
	let first = emoji.as_bytes().first();
	if first.is_some_and(|c| c.is_ascii_digit()) {
		format!("{:x}-20e3.png", first.unwrap())
	} else {
		format!(
			"{}.png",
			emoji
				.chars()
				.map(|c| format!("{:x}", c as u32))
				.collect::<Vec<_>>()
				.join("-")
		)
	}
}

pub fn url_from_unicode_emoji(emoji: &str) -> String {
	format!(
		"https://twemoji.maxcdn.com/v/latest/72x72/{}",
		filename_from_unicode_emoji(emoji)
	)
}

pub fn filename_from_discord_emoji(id: u64, animated: bool) -> String {
	format!("{}.{}", id, if animated { "gif" } else { "png" })
}

pub fn url_from_discord_emoji(id: u64, animated: bool) -> String {
	format!(
		"https://cdn.discordapp.com/emojis/{}",
		filename_from_discord_emoji(id, animated)
	)
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

impl MatchMap for &str {
	fn match_map<'a, F, T>(
		&'a self,
		regex: &Regex,
		f: F,
	) -> std::iter::Map<std::vec::IntoIter<(&'a str, bool)>, F>
	where
		F: FnMut((&'a str, bool)) -> T,
	{
		let mut pieces: Vec<(&str, bool)> = vec![];

		let mut current = 0_usize;
		for regex_match in regex.find_iter(self) {
			let start = regex_match.start();
			let end = regex_match.end();

			if start != current {
				pieces.push((&self[current..start], false));
			}
			current = end;
			pieces.push((&self[regex_match.range()], true));
		}

		if current != self.len() {
			pieces.push((&self[current..], false));
		}

		pieces.into_iter().map(f)
	}
}
