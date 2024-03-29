use super::logger;
use crate::prelude::*;
use async_trait::async_trait;

use serenity::builder::{CreateAttachment, EditRole};
use serenity::model::prelude::*;
use serenity::prelude::*;

use regex::Regex;

pub trait ResultExt<T> {
	fn log_if_err(self, msg: &str);
	fn ok_or_log(self, msg: &str) -> Option<T>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
	fn log_if_err(self, msg: &str) {
		match self {
			Ok(_) => (),
			Err(e) => {
				logger::error_fmt!("{}: {}", msg, e);
			}
		}
	}

	fn ok_or_log(self, msg: &str) -> Option<T> {
		match self {
			Ok(t) => Some(t),
			Err(e) => {
				logger::error_fmt!("{}: {}", msg, e);
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
	async fn guild_cached(&self, ctx: &Context) -> GovanResult;
}

#[async_trait]
impl CacheGuild for Message {
	async fn guild_cached(&self, ctx: &Context) -> GovanResult {
		if self.guild_id.is_some() && self.guild(&ctx.cache).is_none() {
			ctx.http
				.get_guild(
					self.guild_id
						.expect("Guild somehow disappeared in between lines"),
				)
				.await
				.map_err(
					govanerror::error_map!(log fmt = ("Could not get guild information for {} from message {}",
            self.guild_id.unwrap(),
            self.id)),
				)?;
		}

		Ok(())
	}
}

#[async_trait]
impl CacheGuild for GuildChannel {
	async fn guild_cached(&self, ctx: &Context) -> GovanResult {
		if self.guild(ctx).is_none() {
			ctx.http.get_guild(self.guild_id).await.map_err(
				govanerror::error_map!(log fmt = ("Could not get guild information for {} from channel {}",
          self.id,
          self.guild_id)),
			)?;
		}
		Ok(())
	}
}

pub trait MemberExt {
	fn get_unique_role(&self, ctx: &Context) -> GovanResult<Role>;
}

impl MemberExt for Member {
	fn get_unique_role(&self, ctx: &Context) -> GovanResult<Role> {
		let guild = ctx
			.cache
			.guild(self.guild_id)
			.ok_or_else(govanerror::error_lazy!(
				log fmt = ("Guild could not be fetched from user {}", self.user.id),
				user = "Discord is being a little difficult right now"
			))?;

		let mut roles = self.roles(ctx).ok_or_else(govanerror::error_lazy!(
			log fmt = ("Roles could not be fetched from user {}", self.user.id),
			user = "Discord is being a little difficult right now"
		))?;

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

		Err(govanerror::error!(
			log fmt = ("No unique role found on {}", self.user.id),
			user = "You do not have a personal, unique role!"
		))
	}
}

#[async_trait]
pub trait MessageExt {
	async fn reply_report(
		&self,
		cache_http: impl serenity::http::CacheHttp,
		content: impl std::fmt::Display + Send,
	);

	fn any_image(&self) -> Option<String>;
}

#[async_trait]
impl MessageExt for Message {
	async fn reply_report(
		&self,
		cache_http: impl serenity::http::CacheHttp,
		content: impl std::fmt::Display + Send,
	) {
		let reply = content.to_string();
		self.reply(cache_http, reply)
			.await
			.log_if_err(&format!("Could not reply to message {}", self.id));
	}

	fn any_image(&self) -> Option<String> {
		self.attachments
			.first()
			.map(|a| a.url.clone())
			.or_else(|| {
				self.embeds
					.first()
					.and_then(|e| e.image.as_ref().map(|i| i.url.clone()))
			})
			.or_else(|| self.sticker_items.first().and_then(|s| s.image_url()))
	}
}

#[async_trait]
pub trait RoleExt {
	async fn set_icon(&mut self, ctx: &Context, url: &str) -> GovanResult;

	async fn set_unicode_icon(&mut self, ctx: &Context, emoji: &str) -> GovanResult;

	async fn reset_icon(&mut self, ctx: &Context) -> GovanResult;
}

#[async_trait]
impl RoleExt for Role {
	async fn set_icon(&mut self, ctx: &Context, url: &str) -> GovanResult {
		let url = reqwest::Url::parse(url)?;

		let bytes = reqwest::get(url).await?.bytes().await?;
		let bytes = match image::guess_format(&bytes) {
			Ok(image::ImageFormat::Png) => bytes.into_iter().collect::<Vec<_>>(),
			_ => {
				use image::EncodableLayout;
				use image::GenericImageView;
				use image::ImageEncoder;

				let buffer = image::load_from_memory(bytes.as_bytes())?;
				let (w, h) = buffer.dimensions();

				let mut png = Vec::new();
				let encoder = image::codecs::png::PngEncoder::new(&mut png);

				encoder.write_image(buffer.as_bytes(), w, h, buffer.color())?;

				png
			}
		};

		Ok(self
			.edit(
				&ctx,
				EditRole::from_role(self).icon(Some(&CreateAttachment::bytes(bytes, "role.png"))),
			)
			.await?)
	}

	async fn set_unicode_icon(&mut self, ctx: &Context, emoji: &str) -> GovanResult {
		Ok(self
			.edit(
				&ctx,
				EditRole::from_role(self).unicode_emoji(Some(emoji.into())),
			)
			.await?)
	}

	async fn reset_icon(&mut self, ctx: &Context) -> GovanResult {
		Ok(self
			.edit(&ctx, EditRole::from_role(self).icon(None))
			.await?)
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
