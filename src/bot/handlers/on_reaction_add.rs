use crate::bot::bot::Bot;
use crate::util::logging;

use serenity::prelude::*;
use serenity::model::prelude::*;

impl Bot {
  pub async fn on_reaction_add(&self, _ctx: Context, add_reaction: Reaction) {
    if add_reaction.channel_id != 216992217988857857_u64 {
      return;
    }
    // TODO
    logging::debug("Halls and others");
  }
}