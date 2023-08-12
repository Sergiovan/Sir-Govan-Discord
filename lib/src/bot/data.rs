use serenity::client::bridge::gateway::ShardManager;
use serenity::prelude::*;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub mod emoji {
	use super::EmojiType;

	pub const PIN: char = '📌';
	pub const NO_MOUTH: char = '😶';
	pub const WEARY: char = '😩';

	pub const _REPEAT: char = '🔁';
	pub const _REPEAT_ONCE: char = '🔂';
	pub const _VIOLIN: char = '🎻';
	pub const _HEADSTONE: char = '🪦';
	pub const _FIRE_HEART: &str = "❤️‍🔥";

	pub const REDDIT_GOLD: EmojiType = EmojiType::Discord(263774481233870848);
}

pub mod config {
	use crate::bot::data::emoji;
	use serde::{Deserialize, Serialize};
	use serenity::model::prelude::ReactionType;

	pub const DATA_PATH: &str = "res";

	pub const SETTINGS_FILE: &str = "servers.toml";
	pub const NO_CONTEXT_FILE: &str = "nocontext.txt";

	pub const MEDIA_DIR: &str = "media";
	pub const HTML_DIR: &str = "html";
	pub const SAVED_DIR: &str = "saved";

	pub const FALLBACK_IMAGE: &str = "file_missing.png";

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
		pub pin_amount: u32,

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

	pub fn read_servers() -> Result<Servers, ServerTomlError> {
		use std::fs;
		use std::path::Path;

		let settings_path = Path::new(DATA_PATH).join(SETTINGS_FILE);
		let data = fs::read_to_string(settings_path)?;

		let servers: Servers = toml::from_str(&data)?;

		Ok(servers)
	}
}

pub use config::EmojiType;
pub use config::Hall;
pub use config::NoContext;

use crate::util::random;

#[derive(Debug)]
pub struct Channels {
	pub allowed_commands: HashSet<u64>,
	pub disallowed_listen: HashSet<u64>,
}

impl From<config::Channels> for Channels {
	fn from(value: config::Channels) -> Self {
		Channels {
			allowed_commands: HashSet::from_iter(value.allowed_commands.into_iter()),
			disallowed_listen: HashSet::from_iter(value.disallowed_listen.into_iter()),
		}
	}
}

#[derive(Debug)]
pub struct Server {
	pub id: u64,
	pub beta: bool,
	pub nickname: Option<String>,
	pub pin_amount: u32,

	pub channels: Channels,
	pub no_context: Option<NoContext>,

	pub hall_of_fame: Option<Hall<{ emoji::PIN }>>,
	pub hall_of_typo: Option<Hall<{ emoji::WEARY }>>,
	pub hall_of_vague: Option<Hall<{ emoji::NO_MOUTH }>>,
	pub hall_of_all: Option<Hall<'\0'>>,
}

impl Server {
	fn get_emoji<const T: char>(hall: Option<&Hall<T>>) -> Option<EmojiType> {
		hall.map(|x| x.get_emoji())
	}

	pub fn is_fame_emoji(&self, emoji: &EmojiType) -> bool {
		Server::get_emoji(self.hall_of_fame.as_ref()).map_or(false, |x| &x == emoji)
	}

	pub fn is_typo_emoji(&self, emoji: &EmojiType) -> bool {
		Server::get_emoji(self.hall_of_typo.as_ref()).map_or(false, |x| &x == emoji)
	}

	pub fn is_vague_emoji(&self, emoji: &EmojiType) -> bool {
		Server::get_emoji(self.hall_of_vague.as_ref()).map_or(false, |x| &x == emoji)
	}
}

impl From<config::Server> for Server {
	fn from(value: config::Server) -> Self {
		Server {
			id: value.id,
			beta: value.beta,
			nickname: value.nickname,

			channels: value.channels.into(),
			no_context: value.no_context,

			pin_amount: value.pin_amount,
			hall_of_fame: value.hall_of_fame,
			hall_of_typo: value.hall_of_typo,
			hall_of_vague: value.hall_of_vague,
			hall_of_all: value.hall_of_all,
		}
	}
}

pub struct BotData {
	pub servers: HashMap<u64, Server>,
	pub beta: bool,

	no_context_strings: Vec<String>,
}

impl BotData {
	pub fn new(beta: bool) -> BotData {
		BotData {
			servers: HashMap::new(),
			beta,
			no_context_strings: vec![],
		}
	}

	pub fn load_no_context(&mut self) -> Result<(), std::io::Error> {
		use std::fs;
		use std::path::Path;

		let settings_path = Path::new(config::DATA_PATH).join(config::NO_CONTEXT_FILE);
		let data = fs::read_to_string(settings_path)?;
		let data = data.lines();

		self.no_context_strings = data.map(str::to_string).collect();

		Ok(())
	}

	pub fn random_no_context(&self) -> String {
		random::pick_or(&self.no_context_strings, &"No context".to_string()).clone()
	}
}

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
	type Value = Arc<Mutex<ShardManager>>;
}
