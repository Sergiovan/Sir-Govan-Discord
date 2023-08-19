use crate::bot::data::emoji;
use serde::{Deserialize, Serialize};
use serenity::model::prelude::ReactionType;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EmojiType {
	Unicode(String),
	Discord(u64),
}

impl PartialEq<EmojiType> for EmojiType {
	fn eq(&self, other: &EmojiType) -> bool {
		match self {
			EmojiType::Unicode(name) => match other {
				EmojiType::Unicode(oname) => name == oname,
				EmojiType::Discord(..) => false,
			},
			EmojiType::Discord(id) => match other {
				EmojiType::Unicode(_) => false,
				EmojiType::Discord(oid) => id == oid,
			},
		}
	}
}

impl Eq for EmojiType {}

impl From<&ReactionType> for EmojiType {
	fn from(value: &ReactionType) -> Self {
		match value {
			ReactionType::Unicode(name) => EmojiType::Unicode(name.clone()),
			ReactionType::Custom { id, .. } => EmojiType::Discord(*id.as_u64()),
			_ => EmojiType::Discord(0),
		}
	}
}

impl From<&str> for EmojiType {
	fn from(value: &str) -> Self {
		EmojiType::Unicode(value.to_string())
	}
}

impl From<u64> for EmojiType {
	fn from(value: u64) -> Self {
		EmojiType::Discord(value)
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Hall<const N: char> {
	pub channel: u64,
	pub emoji: Option<EmojiType>,
}

impl<const N: char> Hall<N> {
	pub fn get_emoji(&self) -> EmojiType {
		self.emoji
			.clone()
			.unwrap_or(EmojiType::Unicode(N.to_string()))
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NoContext {
	pub channel: u64,
	pub role: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Channels {
	pub allowed_commands: Vec<u64>,
	pub disallowed_listen: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Server {
	pub id: u64,
	pub beta: bool,
	pub nickname: Option<String>,
	pub pin_amount: usize,

	pub channels: Channels,
	pub no_context: Option<NoContext>,

	pub hall_of_fame: Option<Hall<{ emoji::PIN }>>,
	pub hall_of_typo: Option<Hall<{ emoji::WEARY }>>,
	pub hall_of_vague: Option<Hall<{ emoji::NO_MOUTH }>>,
	pub hall_of_all: Option<Hall<'\0'>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Servers {
	pub servers: Vec<Server>,
}

pub enum ServerTomlError {
	IO(std::io::Error),
	Toml(toml::de::Error),
}

impl From<std::io::Error> for ServerTomlError {
	fn from(value: std::io::Error) -> Self {
		ServerTomlError::IO(value)
	}
}

impl From<toml::de::Error> for ServerTomlError {
	fn from(value: toml::de::Error) -> Self {
		ServerTomlError::Toml(value)
	}
}
