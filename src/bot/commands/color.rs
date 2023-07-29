use crate::bot::commands::commander::Commander;
use serenity::prelude::*;
use serenity::model::prelude::*;

impl Commander {
  pub async fn color(&self, _ctx: &Context, _msg: &Message, _words: Vec<&str>) {
    println!("Tired smile emoji");
  }
}