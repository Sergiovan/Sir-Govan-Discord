use crate::bot::Bot;
use crate::util::logging;
use serenity::model::prelude::*;

impl Bot {
  pub async fn on_ready(&self, ready: Ready) {
    logging::info(&format!("Am ready :). I am {}", ready.user.tag()));
  }
}