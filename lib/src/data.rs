pub mod config;
pub mod emoji;
pub mod regex;
pub mod servers;
pub mod strings;

use std::collections::{HashMap, HashSet};

pub use servers::EmojiType;
pub use servers::Hall;
pub use servers::NoContext;

use self::strings::{StringBag, StringBagLoose};

use crate::util::random;

use thiserror::Error;

#[derive(Debug)]
pub struct Channels {
	pub allowed_commands: HashSet<u64>,
	pub disallowed_listen: HashSet<u64>,
}

impl From<servers::Channels> for Channels {
	fn from(value: servers::Channels) -> Self {
		Channels {
			allowed_commands: HashSet::from_iter(value.allowed_commands),
			disallowed_listen: HashSet::from_iter(value.disallowed_listen),
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

#[derive(Debug, Error)]
pub struct StringsConversionError {
	original: random::GrabBagBuilderError,
	string_name: String,
}

impl std::fmt::Display for StringsConversionError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Error while converting {}: {}",
			self.string_name, self.original
		)
	}
}

#[derive(Default)]
pub struct Strings {
	pub nickname: StringBagLoose,

	pub ping: StringBag,

	pub roll: StringBag,

	pub tweet_retweeter: StringBagLoose,
	pub tweet_fact_check: StringBagLoose,
	pub tweet_month: StringBagLoose,
	pub tweet_client: StringBag,
	pub tweet_esoteric_amount_prefix: StringBagLoose,
	pub tweet_esoteric_amount_suffix: StringBagLoose,
	pub tweet_amount_symbol: StringBagLoose,
	pub tweet_esoteric_time: StringBagLoose,
	pub tweet_username: StringBagLoose,
	pub tweet_extra_reply: StringBagLoose,
	pub tweet_extra_text: StringBagLoose,

	pub titlecard_song: StringBag,
	pub titlecard_show_prefix: StringBag,
	pub titlecard_show_entire: StringBag,
}

macro_rules! convert {
	($value:expr, $($iden:ident),*) => {
    Strings {
      $($iden: $value.$iden.try_into().map_err(|e| StringsConversionError{
        original: e, string_name: stringify!($iden).to_string()
      })?,)*
    }
  };
}

impl TryFrom<strings::Strings> for Strings {
	type Error = StringsConversionError;

	fn try_from(value: strings::Strings) -> Result<Self, Self::Error> {
		Ok(convert!(
			value,
			nickname,
			ping,
			roll,
			tweet_retweeter,
			tweet_fact_check,
			tweet_month,
			tweet_client,
			tweet_esoteric_amount_prefix,
			tweet_esoteric_amount_suffix,
			tweet_amount_symbol,
			tweet_esoteric_time,
			tweet_username,
			tweet_extra_reply,
			tweet_extra_text,
			titlecard_song,
			titlecard_show_prefix,
			titlecard_show_entire
		))
	}
}

pub struct BotData {
	pub servers: HashMap<u64, Server>,
	pub beta: bool,
	pub strings: Strings,

	no_context_strings: Vec<String>,
}

impl BotData {
	pub fn new(beta: bool) -> BotData {
		BotData {
			servers: HashMap::new(),
			beta,
			strings: Strings::default(),
			no_context_strings: vec![],
		}
	}

	pub fn load_servers(&mut self) -> anyhow::Result<()> {
		use std::fs;
		use std::path::Path;

		let settings_path = Path::new(config::RESOURCE_PATH).join(config::SETTINGS_FILE);
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

		let settings_path = Path::new(config::RESOURCE_PATH).join(config::NO_CONTEXT_FILE);
		let data = fs::read_to_string(settings_path)?;
		let data = data.lines();

		self.no_context_strings = data.map(str::to_string).collect();

		Ok(())
	}

	pub fn load_strings(&mut self) -> anyhow::Result<()> {
		use std::fs;
		use std::path::Path;

		let strings_path = Path::new(config::RESOURCE_PATH).join(config::STRINGS_FILE);
		let data = fs::read_to_string(strings_path)?;

		let strings: strings::Strings = toml::from_str(&data)?;
		self.strings = strings.try_into()?;

		Ok(())
	}

	pub fn random_no_context(&self) -> String {
		random::pick_or(&self.no_context_strings, &"No context".to_string()).clone()
	}

	pub fn no_context_index(&self, role_name: &str) -> (Option<usize>, usize) {
		(
			self.no_context_strings.iter().position(|r| r == role_name),
			self.no_context_strings.len(),
		)
	}
}
