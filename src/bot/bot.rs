use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::async_trait;
use serenity::json::Value;

use serenity::model::gateway::Ready;
use tracing::debug;

pub struct Bot;

#[async_trait]
impl EventHandler for Bot {
  async fn ready(&self, ctx: Context, ready: Ready) {
    self.on_ready(ctx, ready).await;
  }

  async fn resume(&self, _ctx: Context, _: ResumedEvent) {
    // TODO
    debug!("Reconnected :)");
  }

  async fn unknown(&self, _ctx: Context, _name: String, _raw: Value) {
    // TODO
    debug!("wtf");
  }

  async fn message(&self, ctx: Context, msg: Message) {
    self.on_message(ctx, msg).await;
  }

  async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
    self.on_reaction_add(ctx, add_reaction).await;
  }
}

