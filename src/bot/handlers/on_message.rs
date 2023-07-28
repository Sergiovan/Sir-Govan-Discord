use crate::bot::bot::Bot;
use crate::util::logging;

use serenity::prelude::*;
use serenity::model::prelude::*;

impl Bot {
  pub async fn on_message(&self, _ctx: Context, msg: Message) {
    if msg.channel_id != 216992217988857857_u64 {
      return;
    }
    // TODO
    logging::debug("Message arrived...");
  }
}