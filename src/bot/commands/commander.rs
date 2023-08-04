use crate::bot::data::EmojiType;
use core::future::Future;
use std::{collections::HashMap, pin::Pin};

use regex::Regex;

use num_bigint::BigInt;
use once_cell::sync::Lazy;
use serenity::model::prelude::*;
use serenity::prelude::*;

use std::collections::VecDeque;

type CommandRet<'a> = Pin<Box<(dyn Future<Output = ()> + 'a + Send)>>;
type CommandFn = (dyn for<'fut> Fn(&'fut Commander, &'fut Context, &'fut Message, Vec<&'fut str>) -> CommandRet<'fut>
     + Sync
     + Send);
type Command = Box<CommandFn>;

pub struct Commander {
    commands: HashMap<String, Command>,
}

// You know what, close enough
macro_rules! command {
    ($function:expr) => {
        Box::new(|s, c, m, v| Box::pin($function(s, c, m, v)))
    };
}

impl Commander {
    pub fn new() -> Commander {
        Commander {
            commands: HashMap::new(),
        }
    }

    pub fn register_all(&mut self) {
        self.register_command("color", command!(Self::color));
    }

    pub fn register_command(&mut self, name: &str, command: Command) {
        self.commands.insert(name.to_string(), command);
    }

    pub async fn parse(&self, ctx: &Context, msg: &Message) {
        if !msg.content.starts_with('!') {
            return;
        }

        let words = msg.content.split_whitespace().collect::<Vec<_>>();
        if words.is_empty() {
            return;
        }

        let first: &str = &words[0][1..];

        if let Some(f) = self.commands.get(first) {
            f(self, ctx, msg, words).await;
        }
    }
}

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

// TODO Remove
#[allow(dead_code)]
impl<'a> Arguments<'a> {
    pub fn count(&self) -> usize {
        self.args.len()
    }

    fn shift(&mut self) {
        self.args.pop_front();
    }

    pub fn try_arg(&mut self) -> Option<Argument<'a>> {
        static IS_EMOJI: Lazy<Regex> = Lazy::new(|| {
            {
                Regex::new(concat!(
                    "^[",
                    "\u{01F600}-\u{01F64F}", // emoticons
                    "\u{01F300}-\u{01F5FF}", // symbols & pictographs
                    "\u{01F680}-\u{01F6FF}", // transport & map symbols
                    "\u{01F1E0}-\u{01F1FF}", // flags (iOS)
                    "\u{002702}-\u{0027B0}",
                    "\u{0024C2}-\u{01F251}",
                    "]+$",
                ))
            }
            .expect("Regex failed")
        });

        let arg = self.args.front()?;

        let chars = arg.chars().collect::<Vec<_>>();
        let first = chars.first();

        match first {
            Some('0'..='9') => match arg.parse::<BigInt>() {
                Ok(number) => {
                    if number < 0.into() || number > u64::MAX.into() {
                        Some(Argument::BigNumber(number))
                    } else {
                        match arg.parse::<u64>() {
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
            Some(_) if IS_EMOJI.is_match(arg) => {
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
        let EmojiType::Discord(emoji) = self.emoji()? else { return None };
        ctx.cache
            .guild(guild_id)
            .and_then(|g| g.emojis.get(&emoji.into()).cloned())
    }
}
