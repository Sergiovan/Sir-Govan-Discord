use crate::bot::data::{BotData, EmojiType};
use crate::bot::{Bot, CacheGuild};
use crate::util::logger;

use serenity::model::prelude::*;
use serenity::prelude::*;

impl Bot {
    pub async fn on_reaction_add(&self, ctx: Context, add_reaction: Reaction) {
        let reactor = match add_reaction.user(&ctx).await {
            Ok(reactor) => reactor,
            Err(e) => {
                logger::error(&format!(
                    "Could not determine reactor for reaction {:?}: {}",
                    add_reaction, e
                ));
                return;
            }
        };

        if reactor.id == ctx.cache.current_user_id() {
            return;
        }

        let msg = match add_reaction.message(&ctx.http).await {
            Ok(msg) => msg,
            Err(err) => {
                logger::error(&format!(
                    "Message {} that was reacted to with {} could not be fetched: {}",
                    add_reaction.message_id, add_reaction.emoji, err
                ));
                return;
            }
        };

        if !msg.guild_cached(&ctx).await {
            return;
        }

        if msg.is_own(&ctx) {
            return;
        }

        // So, msg.is_private() won't work because messages fetched through the REST API don't come with
        // a guild_id, which means msg.is_private() will always be true
        let this_channel = match msg.channel(&ctx).await {
            Ok(channel) => channel,
            Err(e) => {
                logger::error(&format!(
                    "Message {}'s channel {} could not be fetched: {}",
                    msg.id, msg.channel_id, e
                ));
                return;
            }
        };

        let this_channel = match this_channel.guild() {
            Some(channel) => channel,
            None => return, // No error message, this is valid
        };

        enum Action {
            Pin,
            None,
        }

        let (action, destination_id, required): (Action, u64, u32) = {
            let data = ctx.data.read().await;
            let bot_data = data.get::<BotData>().unwrap().read().await;

            let server = match bot_data.servers.get(this_channel.guild_id.as_u64()) {
                Some(server) => server,
                None => return,
            };

            if server
                .channels
                .disallowed_listen
                .contains(msg.channel_id.as_u64())
            {
                return;
            }

            let emoji: EmojiType = (&add_reaction.emoji).into();
            if server.is_fame_emoji(&emoji) {
                let hall = server.hall_of_fame.as_ref().unwrap();
                let channel_id = hall.channel;
                let required = server.pin_amount;

                (Action::Pin, channel_id, required)
            } else {
                (Action::None, 0, u32::MAX)
            }
        };

        if matches!(action, Action::None) || destination_id == 0 {
            return;
        }

        let channel = match ctx.cache.guild_channel(destination_id) {
            Some(channel) => channel,
            None => {
                match ctx.http.get_channel(destination_id).await {
                    Ok(Channel::Guild(channel)) => channel,
                    Ok(c) => {
                        logger::error(&format!("Channel {} for hall emoji {} is misconfigured, not a guild channel: {}", destination_id, add_reaction.emoji, c));
                        return;
                    }
                    Err(e) => {
                        logger::error(&format!(
                            "Error when fetching channel {}: {}",
                            destination_id, e
                        ));
                        return;
                    }
                }
            }
        };

        if !channel.guild_cached(&ctx).await {
            return;
        }

        match action {
            Action::Pin => {
                self.maybe_pin(
                    ctx,
                    msg,
                    add_reaction,
                    channel,
                    required,
                    Some(crate::bot::data::emoji::REDDIT_GOLD),
                )
                .await
            }
            Action::None => (),
        };
    }
}
