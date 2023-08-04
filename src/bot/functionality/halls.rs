use crate::bot::bot::Bot;

use serenity::model::prelude::*;
use serenity::prelude::*;

pub struct HallSafety;

impl HallSafety {
    pub async fn can_pin(&self, ctx: Context, reaction: Reaction) -> bool {
        return false;
    }
}

impl Bot {
    pub async fn maybe_pin(&self, ctx: Context, reaction: Reaction, dest: GuildChannel) {
        println!("Yep");
    }
}
