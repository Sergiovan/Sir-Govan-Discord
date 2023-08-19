pub mod config;
pub mod emoji;
pub mod servers;

use std::collections::{HashMap, HashSet};

pub use servers::EmojiType;
pub use servers::Hall;
pub use servers::NoContext;

use crate::util::random;

#[derive(Debug)]
pub struct Channels {
	pub allowed_commands: HashSet<u64>,
	pub disallowed_listen: HashSet<u64>,
}

impl From<servers::Channels> for Channels {
	fn from(value: servers::Channels) -> Self {
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
	pub pin_amount: usize,

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

impl From<servers::Server> for Server {
	fn from(value: servers::Server) -> Self {
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

	pub fn load_servers(&mut self) -> anyhow::Result<()> {
		use std::fs;
		use std::path::Path;

		let settings_path = Path::new(config::DATA_PATH).join(config::SETTINGS_FILE);
		let data = fs::read_to_string(settings_path)?;

		let servers: servers::Servers = toml::from_str(&data)?;

		self.servers.extend(
			servers
				.servers
				.into_iter()
				.filter(|server| server.beta == self.beta)
				.map(|server| (server.id, server.into())),
		);

		Ok(())
	}

	pub fn load_no_context(&mut self) -> anyhow::Result<()> {
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
