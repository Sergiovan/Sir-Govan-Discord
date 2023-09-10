use crate::prelude::*;

use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::bot::Bot;
use crate::data::Server;

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
	) -> GovanResult {
		let misconfigured_error = govanerror::error_lazy!(
			log fmt = ("Server misconfigured: {}", server.id),
			user = "< This guy's caretaker dun goof'd"
		);
		let no_context = server.no_context.as_ref().ok_or_else(misconfigured_error)?;

		let channel = ctx
			.cache
			.guild_channel(no_context.channel)
			.ok_or_else(misconfigured_error)?;

		let role = ctx
			.cache
			.role(channel.guild_id, no_context.role)
			.ok_or_else(misconfigured_error)?;

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
			.await?;

		use serenity::futures::StreamExt;

		let members = channel.guild_id.members_iter(ctx);
		let members = members.collect::<Vec<_>>().await;

		for member in members.into_iter() {
			let mut member = member?;

			let id = member.user.id;
			if id == msg.author.id {
				member.add_role(&ctx, role.id).await?;
			} else {
				member.remove_role(&ctx, role.id).await?;
			}
		}

		let new_role_name = self.data().await.random_no_context();
		role.edit(&ctx, |r| r.name(&new_role_name)).await?;

		Ok(())
	}
}
