use crate::prelude::*;

use crate::bot::Bot;
use crate::data::EmojiType;
use std::collections::HashMap;

use num_bigint::BigInt;
use serenity::model::prelude::*;
use serenity::prelude::*;

use async_trait::async_trait;

use std::collections::VecDeque;

#[async_trait]
pub trait Command: Send + Sync {
	fn name() -> &'static str
	where
		Self: Sized;

	fn aliases() -> &'static [&'static str]
	where
		Self: Sized,
	{
		&[]
	}

	async fn execute<'a>(
		&self,
		ctx: &Context,
		msg: &'a Message,
		mut args: Arguments<'a>,
		bot: &Bot,
	) -> GovanResult;
}

pub struct Commander {
	commands: HashMap<String, &'static dyn Command>,
}

impl Default for Commander {
	fn default() -> Self {
		Commander::new()
	}
}

impl Commander {
	pub fn new() -> Commander {
		Commander {
			commands: HashMap::new(),
		}
	}

	pub fn register_all(&mut self) {
		self.register_command(&super::color::Color);
		self.register_command(&super::quit::Quit);
		self.register_command(&super::role::Role);
		self.register_command(&super::icon::Icon);
		self.register_command(&super::roll::Roll);
		self.register_command(&super::ping::Ping);
	}

	pub fn register_command<T: Command + 'static>(&mut self, command: &'static T) {
		self.commands.insert(format!("!{}", T::name()), command);
		for alias in T::aliases().iter() {
			self.commands.insert(format!("!{}", alias), command);
		}
	}

	pub async fn parse(&self, ctx: &Context, msg: &Message, bot: &Bot) -> GovanResult {
		if !msg.content.starts_with('!') {
			return Err(govanerror::debug!(log = "No such command"));
		}

		let content = msg.content.clone();
		let mut words = Arguments::from(content.as_str());

		if words.empty() {
			return Err(govanerror::debug!(
				log = "Message could not be converted to arguments"
			));
		}

		let first: &str = words
			.string()
			.expect("Non-empty arguments didn't return string");

		if let Some(c) = self.commands.get(first) {
			Ok(c.execute(ctx, msg, words, bot).await?)
		} else {
			Err(govanerror::debug!(log = "No such command"))
		}
	}
}

#[allow(dead_code)]
pub enum Argument<'a> {
	String(&'a str),
	Number(u64),
	BigNumber(BigInt),
	Channel(u64),
	Role(u64),
	User(u64),
	Emoji(EmojiType),
}

pub struct Arguments<'a> {
	args: VecDeque<&'a str>,
}

impl<'a, 'b: 'a> From<&'b str> for Arguments<'a> {
	fn from(value: &'b str) -> Arguments<'a> {
		Arguments {
			args: value[..].split_whitespace().collect::<VecDeque<_>>(),
		}
	}
}

#[allow(dead_code)]
impl<'a> Arguments<'a> {
	pub fn count(&self) -> usize {
		self.args.len()
	}

	pub fn empty(&self) -> bool {
		self.count() == 0
	}

	fn shift(&mut self) {
		self.args.pop_front();
	}

	pub fn try_arg(&mut self) -> Option<Argument<'a>> {
		let arg = self.args.front()?;

		let chars = arg.chars().collect::<Vec<_>>();
		let first = chars.first();

		match first {
			Some('0'..='9') | Some('-') | Some('+') => match arg.parse::<BigInt>() {
				Ok(number) => {
					if number < 0.into() || number > u64::MAX.into() {
						Some(Argument::BigNumber(number))
					} else {
						match arg.replace('-', "").parse::<u64>() {
							Ok(number) => Some(Argument::Number(number)),
							Err(_) => Some(Argument::String(arg)),
						}
					}
				}
				Err(_) => Some(Argument::String(arg)),
			}, // Maybe number
			Some('<') if chars.last().is_some_and(|x| x == &'>') => {
				let second = chars.get(1);
				match second {
					Some('@') => {
						let third = chars.get(2);
						match third {
							Some('&') => {
								let maybe_id = &arg[3..arg.len() - 1];
								match maybe_id.parse::<u64>() {
									Ok(num) => Some(Argument::Role(num)),
									Err(_) => Some(Argument::String(arg)),
								}
							}
							Some('>') => Some(Argument::String(arg)),
							Some(_) => {
								let maybe_id = &arg[2..arg.len() - 1];
								match maybe_id.parse::<u64>() {
									Ok(num) => Some(Argument::User(num)),
									Err(_) => Some(Argument::String(arg)),
								}
							}
							None => unreachable!(), // We have to find a >
						}
					}
					Some(':') => {
						if let Some(pos) = arg.rfind(':') {
							let maybe_id = &arg[pos + 1..arg.len() - 1];
							match maybe_id.parse::<u64>() {
								Ok(num) => Some(Argument::Emoji(EmojiType::Discord(num))),
								Err(_) => Some(Argument::String(arg)),
							}
						} else {
							Some(Argument::String(arg))
						}
					}
					Some('#') => {
						let maybe_id = &arg[2..arg.len() - 1];
						match maybe_id.parse::<u64>() {
							Ok(num) => Some(Argument::Channel(num)),
							Err(_) => Some(Argument::String(arg)),
						}
					}
					Some(_) | None => Some(Argument::String(arg)),
				}
			} // Maybe Channel, Role, User
			Some(_) if crate::data::regex::EMOJI_REGEX.is_match(arg) => {
				Some(Argument::Emoji(EmojiType::Unicode(arg.to_string())))
			}
			Some(_) => Some(Argument::String(arg)),
			None => None,
		}
	}

	pub fn arg(&mut self) -> Option<Argument<'a>> {
		let ret = self.try_arg();
		self.shift();
		ret
	}

	pub fn string(&mut self) -> Option<&'a str> {
		self.args.pop_front()
	}

	pub fn rest(mut self) -> String {
		self.args.make_contiguous().join(" ")
	}

	pub fn number(&mut self) -> Option<u64> {
		if let Argument::Number(num) = self.try_arg()? {
			self.shift();
			Some(num)
		} else {
			None
		}
	}

	pub fn big_number(&mut self) -> Option<BigInt> {
		match self.try_arg()? {
			Argument::Number(num) => {
				self.shift();
				Some(num.into())
			}
			Argument::BigNumber(num) => {
				self.shift();
				Some(num)
			}
			_ => None,
		}
	}

	pub fn channel_id(&mut self) -> Option<u64> {
		if let Argument::Channel(ch) = self.try_arg()? {
			self.shift();
			Some(ch)
		} else {
			None
		}
	}

	pub fn channel(&mut self, ctx: &Context, guild_id: u64) -> Option<GuildChannel> {
		let ch = self.channel_id()?;
		ctx.cache
			.guild_channel(ch)
			.filter(|x| x.guild_id == guild_id)
	}

	pub fn user_id(&mut self) -> Option<u64> {
		if let Argument::User(u) = self.try_arg()? {
			self.shift();
			Some(u)
		} else {
			None
		}
	}

	pub fn user(&mut self, ctx: &Context, guild_id: u64) -> Option<Member> {
		let user = self.user_id()?;
		ctx.cache.member(guild_id, user)
	}

	pub fn role_id(&mut self) -> Option<u64> {
		if let Argument::Role(r) = self.try_arg()? {
			self.shift();
			Some(r)
		} else {
			None
		}
	}

	pub fn role(&mut self, ctx: &Context, guild_id: u64) -> Option<Role> {
		let role = self.role_id()?;
		ctx.cache.role(guild_id, role)
	}

	pub fn emoji(&mut self) -> Option<EmojiType> {
		if let Argument::Emoji(e) = self.try_arg()? {
			self.shift();
			Some(e)
		} else {
			None
		}
	}

	pub fn guild_emoji(&mut self, ctx: &Context, guild_id: u64) -> Option<Emoji> {
		let EmojiType::Discord(emoji) = self.emoji()? else {
			return None;
		};
		ctx.cache
			.guild(guild_id)
			.and_then(|g| g.emojis.get(&emoji.into()).cloned())
	}
}
