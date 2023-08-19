use std::convert::Infallible;

use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::bot::Bot;
use crate::data::Server;
use crate::util::{NickOrName, ResultErrorHandler};

impl Bot {
	pub fn can_remove_context(&self, ctx: &Context, msg: &Message, server: &Server) -> bool {
		server.no_context.as_ref().is_some_and(|nc| {
			ctx.cache.guild_channel(nc.channel).is_some_and(|c| {
				c.guild_id == server.id
					&& c.permissions_for_user(ctx, ctx.cache.current_user())
						.is_ok_and(|p| p.send_messages())
			}) && ctx.cache.role(server.id, nc.role).is_some()
		}) && msg.content.len() <= 280
	}

	pub async fn remove_context(
		&self,
		ctx: &Context,
		msg: &Message,
		server: &Server,
	) -> Option<Infallible> {
		let no_context = server
			.no_context
			.as_ref()
			.expect("Server didn't have no-context");
		let channel = ctx
			.cache
			.guild_channel(no_context.channel)
			.expect("Channel didn't exist");
		let role = ctx
			.cache
			.role(channel.guild_id, no_context.role)
			.expect("Role didn't exist");

		channel
			.send_message(&ctx, |b| {
				msg.attachments.iter().for_each(|a| {
					b.add_file(a.url.as_str());
				});

				msg.sticker_items.iter().for_each(|s| {
					b.add_sticker_id(s.id);
				});

				b.content(&msg.content)
			})
			.await
			.ok_or_log(&format!(
				"Could not post message {} to {}",
				msg.id, channel.name
			))?;

		for (id, member) in channel.guild(ctx).unwrap().members.iter_mut() {
			if id == &msg.author.id {
				member.add_role(&ctx, role.id).await.log_if_err(&format!(
					"Could not add no-context role {} to user {}",
					role.id,
					member.get_name()
				));
			} else {
				member.remove_role(&ctx, role.id).await.log_if_err(&format!(
					"Could not remove no-context role {} from user {}",
					role.id,
					member.get_name()
				));
			}
		}

		let new_role_name = self.data.read().await.random_no_context();
		role.edit(&ctx, |r| r.name(&new_role_name))
			.await
			.log_if_err(&format!(
				"Could not rename no-context role {} from {} to {}",
				role.id, role.name, new_role_name
			));

		None
	}
}
