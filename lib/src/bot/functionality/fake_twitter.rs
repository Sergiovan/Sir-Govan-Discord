use std::convert::Infallible;

use crate::bot::Bot;

use crate::bot::helpers::handlebars::TweetData;
use crate::util::{OptionErrorHandler, ResultErrorHandler};

use serenity::model::prelude::*;
use serenity::prelude::*;

use itertools::Itertools;

impl Bot {
	async fn tweet_data_from_message(
		&self,
		ctx: &Context,
		messages: &[Message],
		reaction: Reaction,
		verified_role: Option<u64>,
	) -> Option<TweetData> {
		let attachment = messages.iter().find_map(|msg| {
			msg.attachments.first().map(|a| a.url.clone()).or(msg
				.embeds
				.first()
				.and_then(|e| e.image.as_ref().map(|i| i.url.clone())))
		});
		let content = messages
			.iter()
			.map(|msg| msg.content.clone())
			.filter(|content| &attachment.as_ref().unwrap_or(&String::new()) != &content)
			.collect_vec();

		let first = messages.first().unwrap();

		let member = first.member(&ctx).await.ok_or_log(&format!(
			"Could not get member data from {}",
			first.author.id
		))?;

		Some(TweetData {
			retweeter: reaction
				.user(&ctx)
				.await
				.ok_or_log(&format!(
					"Could not get user for reaction on {}",
					reaction.message_id
				))?
				.name,
			avatar: member.face(),
			name: member.display_name().into_owned(),
			verified: verified_role.is_none()
				|| verified_role.is_some_and(|id| member.roles.iter().any(|&r| r == id)),
			at: member.user.name,
			tweet_text: todo!(),
			hour: todo!(),
			month: todo!(),
			day: todo!(),
			year: todo!(),

			client: todo!(),
			any_numbers: todo!(),
			retweets: todo!(),
			quotes: todo!(),
			likes: todo!(),
			more_tweets: todo!(),

			theme: todo!(),
			reply_to: todo!(),
			image: todo!(),
			fact_check: todo!(),
		})
	}

	pub async fn maybe_retweet(
		&self,
		ctx: Context,
		msg: Message,
		reaction: Reaction,
		with_context: bool,
		verified_role: Option<u64>,
	) -> Option<Infallible> {
		let screenshotter = self.get_screenshotter().await;
		let screenshotter = screenshotter.as_ref();

		if screenshotter.is_none() {
			msg.reply(&ctx, "I could not connect to the Infinitely Tall Cylinder Earth Twitter servers. Please try again later")
        .await.ok_or_log(&format!("Could not reply to {}", msg.id))?;
		}

		// List must be reversed
		let messages = msg
			.channel(&ctx)
			.await
			.ok_or_log("Could not fetch message channels")?
			.guild()
			.log_if_none("Message was not in guild")?
			.messages(&ctx, |b| b.after(msg.id.0 - 1).limit(50))
			.await
			.ok_or_log("Contextual messages could not be fetched")?;

		if messages.is_empty() {
			msg.reply(&ctx, "I could not access the Infinitely Tall Cylinder Earth Twitter API. Please try again later")
        .await.ok_or_log(&format!("Could not reply to {}", msg.id))?;
		}

		let context = messages
			.into_iter()
			.rev()
			.group_by(|e| e.author.id)
			.into_iter()
			.map(|i| i.1.collect_vec())
			.collect_vec();

		let first = context.first().log_if_none("No messages found")?;

		let tweet_data = self
			.tweet_data_from_message(&ctx, first, reaction, verified_role)
			.await
			.log_if_none("Error creating data from message");

		if tweet_data.is_none() {}

		for rest in context.iter().skip(1) {}

		None
	}
}
