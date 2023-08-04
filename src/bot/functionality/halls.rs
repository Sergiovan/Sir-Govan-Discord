use crate::bot::Bot;

use serenity::model::prelude::*;
use serenity::prelude::*;

pub struct HallSafety;

impl HallSafety {
    pub async fn _can_pin(&self, _ctx: Context, _reaction: Reaction) -> bool {
        false
    }
}

impl Bot {
    pub async fn maybe_pin(&self, _ctx: Context, _reaction: Reaction, _dest: GuildChannel) {
        println!("Yep");
    }
}
