use core::future::Future;
use std::{collections::HashMap, pin::Pin};

use serenity::model::prelude::*;
use serenity::prelude::*;

type CommandRet<'a> = Pin<Box<(dyn Future<Output = ()> + 'a + Send)>>;
type CommandFn = (dyn for<'fut> Fn(&'fut Commander, &'fut Context, &'fut Message, Vec<&'fut str>) -> CommandRet<'fut>
     + Sync
     + Send);
type Command = Box<CommandFn>;

pub struct Commander {
    commands: HashMap<String, Command>,
}

// You know what, close enough
macro_rules! command {
    ($function:expr) => {
        Box::new(|s, c, m, v| Box::pin($function(s, c, m, v)))
    };
}

impl Commander {
    pub fn new() -> Commander {
        Commander {
            commands: HashMap::new(),
        }
    }

    pub fn register_all(&mut self) {
        self.register_command("color", command!(Self::color));
    }

    pub fn register_command(&mut self, name: &str, command: Command) {
        self.commands.insert(name.to_string(), command);
    }

    pub async fn parse(&self, ctx: &Context, msg: &Message) {
        if !msg.content.starts_with('!') {
            return;
        }

        let words = msg.content.split_whitespace().collect::<Vec<_>>();
        if words.is_empty() {
            return;
        }

        let first: &str = &words[0][1..];

        if let Some(f) = self.commands.get(first) {
            f(self, ctx, msg, words).await;
        } else {
            return;
        }
    }
}
