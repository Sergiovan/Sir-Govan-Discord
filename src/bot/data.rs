use serenity::client::bridge::gateway::ShardManager;
use serenity::prelude::*;

use std::collections::{HashSet, HashMap};
use std::sync::Arc;

pub mod config {
  use serde::{Serialize, Deserialize};

  pub const DATA_PATH: &str = "data";
  pub const SETTINGS_FILE: &str = "servers.toml";

  #[derive(Serialize, Deserialize, Debug)]
  pub enum EmojiType {
    Unicode(String),
    Discord(u64),
    DiscordAnimated(u64)
  }

  #[derive(Serialize, Deserialize, Debug)]
  pub struct Hall {
    pub channel: u64,
    pub emoji: Option<EmojiType>
  }

  #[derive(Serialize, Deserialize, Debug)]
  pub struct NoContext {
    pub channel: u64,
    pub role: u64,
  }

  #[derive(Serialize, Deserialize, Debug)]
  pub struct Channels {
    pub allowed_commands: Vec<u64>,
    pub disallowed_listen: Vec<u64>
  }

  #[derive(Serialize, Deserialize, Debug)]
  pub struct Server {
    pub id: u64,
    pub beta: bool,
    pub nickname: Option<String>,
    pub pin_amount: u32,
  
    pub channels: Channels,
    pub no_context: Option<NoContext>,

    pub hall_of_fame: Option<Hall>,
    pub hall_of_typo: Option<Hall>,
    pub hall_of_vague: Option<Hall>,
    pub hall_of_all: Option<Hall>,
  
  }

  #[derive(Serialize, Deserialize, Debug)]
  pub struct Servers {
    pub servers: Vec<Server>
  }

  pub enum Error {
    IO(std::io::Error),
    Toml(toml::de::Error)
  }

  impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IO(value)
    }
  }

  impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Error::Toml(value)
    }
  }

  pub fn read_servers() -> Result<Servers, Error> {
    use std::path::Path;
    use std::fs;

    let settings_path = Path::new(DATA_PATH).join(SETTINGS_FILE);
    let data = fs::read_to_string(&settings_path)?;

    let servers: Servers = toml::from_str(&data)?;

    Ok(servers)
  }
}

pub use config::EmojiType;
pub use config::Hall;
pub use config::NoContext;

#[derive(Debug)]
pub struct Channels {
  pub allowed_commands: HashSet<u64>,
  pub disallowed_listen: HashSet<u64>
}

impl From<config::Channels> for Channels {
  fn from(value: config::Channels) -> Self {
    Channels {
      allowed_commands: HashSet::from_iter(value.allowed_commands.into_iter()),
      disallowed_listen: HashSet::from_iter(value.disallowed_listen.into_iter())
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
  
  pub hall_of_fame: Option<Hall>,
  pub hall_of_typo: Option<Hall>,
  pub hall_of_vague: Option<Hall>,
  pub hall_of_all: Option<Hall>
}

impl Server {

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
      hall_of_all: value.hall_of_all
    }
  }
}

pub struct BotData {
  pub servers: HashMap<u64, Server>,
  pub beta: bool
}

impl BotData {
  pub fn new(beta: bool) -> BotData {
    BotData {
      servers: HashMap::new(),
      beta
    }
  }
}

impl TypeMapKey for BotData {
  type Value = Arc<RwLock<BotData>>;
}

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}