use crate::prelude::*;

use serenity::builder::{CreateAttachment, CreateMessage, EditRole};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::bot::Bot;
use crate::data::Server;

impl Bot {
	pub fn can_remove_context(&self, ctx: &Context, msg: &Message, server: &Server) -> bool {
		server.no_context.as_ref().is_some_and(|nc| {
			nc.channel != 0
				&& ctx.cache.channel(nc.channel).is_some_and(|c| {
					c.guild_id == server.id
						&& c.permissions_for_user(ctx, ctx.cache.current_user().id)
							.is_ok_and(|p| p.send_messages())
				}) && nc.role != 0
				&& ctx.cache.role(server.id, nc.role).is_some()
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

		let channel = ChannelId::new(no_context.channel)
			.to_channel(&ctx)
			.await?
			.guild()
			.ok_or_else(misconfigured_error)?;

		let mut role = ctx
			.cache
			.role(channel.guild_id, no_context.role)
			.ok_or_else(misconfigured_error)?
			.clone();

		let mut b = CreateMessage::default();
		for attachment in msg.attachments.iter() {
			let file = CreateAttachment::url(&ctx, attachment.url.as_str()).await?;
			b = b.add_file(file);
		}

		for sticker in msg.sticker_items.iter() {
			b = b.add_sticker_id(sticker.id);
		}

		b = b.content(&msg.content);

		channel.send_message(&ctx, b).await?;

		use serenity::futures::StreamExt;

		let members = channel.guild_id.members_iter(ctx);
		let members = members.collect::<Vec<_>>().await;

		for member in members.into_iter() {
			let member = member?;

			let id = member.user.id;
			if id == msg.author.id {
				if !member.roles.contains(&role.id) {
					member.add_role(&ctx, role.id).await?;
				}
			} else if member.roles.contains(&role.id) {
				member.remove_role(&ctx, role.id).await?;
			}
		}

		let new_role_name = self.data().await.random_no_context();
		role.edit(&ctx, EditRole::default().name(&new_role_name))
			.await?;

		Ok(())
	}
}
