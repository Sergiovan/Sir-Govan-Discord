use crate::bot::bot::Bot;

use serenity::prelude::*;
use serenity::model::prelude::*;

impl Bot {
  pub async fn maybe_pin(&self, ctx: Context, reaction: Reaction, dest: GuildChannel) {
    println!("Yep");
  }
}