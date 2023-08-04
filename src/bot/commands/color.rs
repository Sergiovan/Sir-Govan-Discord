use crate::bot::commands::commander::Commander;
use serenity::model::prelude::*;
use serenity::prelude::*;

impl Commander {
    pub async fn color(&self, _ctx: &Context, _msg: &Message, _words: Vec<&str>) {
        println!("Tired smile emoji");
    }
}
