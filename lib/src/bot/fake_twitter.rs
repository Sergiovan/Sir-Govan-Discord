use crate::helpers::discord_content_conversion::{ContentConverter, ContentOriginal};
use crate::prelude::*;

use crate::bot::Bot;

use crate::helpers::handlebars::{TweetData, TweetMoreData};

use chrono::DateTime;
use itertools::Itertools;
use num_rational::Ratio;
use serenity::builder::{CreateAllowedMentions, CreateAttachment, CreateMessage, GetMessages};
use serenity::model::prelude::*;
use serenity::prelude::*;

use lazy_static::lazy_static;
use regex::Regex;

async fn content_from_msgs(msgs: &[Message], ctx: &Context, filter: &str) -> GovanResult<String> {
	use html_escape::encode_quoted_attribute as html_encode;

	lazy_static! {
		static ref DISCORD_TAG_SAVER: Regex =
			Regex::new(r"<(a?:(?P<NAME>.*?):|@&?|#)(?P<ID>[0-9]+)>").unwrap();
		static ref TAG_REVERSAL: Regex = Regex::new(r"[\x00|\x01|\x02]").unwrap();
	}

	fn linkify(s: String) -> String {
		format!(r#"<span class="twitter-link">{}</span>"#, s)
	}

	async fn stringify_content(ctx: &Context, content: ContentOriginal) -> String {
		match content {
			ContentOriginal::User(id) => linkify(format!(
				"@{}",
				id.to_user(&ctx)
					.await
					.map_or("Unknown User".to_string(), |u| u.name)
			)),
			ContentOriginal::Channel(id) => linkify(format!(
				"#{}",
				id.to_channel(&ctx)
					.await
					.map_or("Unknown Channel".to_string(), |c| c
						.guild()
						.map_or("Unknown Channel".to_string(), |c| c.name))
			)),
			ContentOriginal::Role(id) => linkify(format!(
				"@{}",
				util::role_from_id(id, ctx).map_or("@Unknown Role".to_string(), |role| role.name)
			)),
			ContentOriginal::Emoji(id) => format!(
				r#"<img class="emoji" src="{}">"#,
				util::url_from_discord_emoji(id.into(), false)
			),
		}
	}

	let content = msgs
		.iter()
		.filter(|msg| filter != msg.content)
		.map(|msg| msg.content.clone())
		.collect_vec()
		.join("\n");

	let mut converter = ContentConverter::new(content)
		.user()
		.channel()
		.emoji()
		.role();

	let ids = converter.take()?;
	let futures = ids.into_iter().map(|e| stringify_content(ctx, e));
	let replacements = util::collect_async(futures).await;

	let replacements = replacements.into_iter().collect::<Vec<_>>();
	converter.transform(|s| html_encode(&s).to_string());
	converter.transform(|s| {
		data::regex::DISCORD_URL
			.replace_all(&s, r#"<span class="twitter-link">$0</span>"#)
			.to_string()
	});
	converter.replace(&replacements)?;

	let content = converter.finish();

	let content = data::regex::EMOJI_REGEX.replace_all(&content, |capture: &regex::Captures| {
		let emoji = capture.get(0).unwrap().as_str();
		match emoji.chars().next().unwrap() {
			'©' => return emoji.to_string(),
			'®' => return emoji.to_string(),
			'™' => return emoji.to_string(),
			_ => (),
		}
		format!(
			r#"<img class="emoji" src="{}">"#,
			util::url_from_unicode_emoji(emoji)
		)
	});

	Ok(content.replace('\n', "<br>"))
}

fn twitter_random_number(strings: &data::Strings) -> Option<String> {
	let num = 'num: {
		let mut rand = util::random::from_range(-0.25..0.75);
		if rand < 0_f64 {
			break 'num 0_f64;
		}
		rand *= 1_f64 / 0.75;
		rand += 1_f64;
		rand = rand.powf(13.2875681028);
		rand.floor()
	};

	let symbol = strings.tweet_amount_symbol.pick_biased(Ratio::new(1, 5));

	if num == 0_f64 {
		None
	} else if symbol.is_none() {
		Some(num.to_string())
	} else {
		let str = num.to_string().chars().take(4).join("");
		if str.len() < 4 || str.chars().last().is_some_and(|c| c == '0') {
			Some(format!(
				"{}{}",
				str.chars().take(3).join(""),
				symbol.unwrap()
			))
		} else {
			Some(format!("{}.{}{}", &str[..3], &str[3..], symbol.unwrap()))
		}
	}
}

impl Bot {
	async fn tweet_data_from_message(
		&self,
		ctx: &Context,
		messages: &[Message],
		reaction: &Reaction,
		verified_role: Option<u64>,
	) -> GovanResult<TweetData> {
		let guild_id = reaction.guild_id.ok_or_else(govanerror::debug_lazy!(
			log = "Not currently in a guild channel",
			user = "You're not in a guild, goofball!"
		))?;

		let attachment = messages.iter().find_map(|msg| msg.any_image());

		let content =
			content_from_msgs(messages, ctx, attachment.as_ref().unwrap_or(&String::new())).await?;

		let first = messages.first().unwrap();

		let member = first.member(&ctx).await?;

		let strings = &self.data().await.strings;
		let retweeter_user = reaction.user(&ctx).await?;

		let retweeter = guild_id
			.member(&ctx, &retweeter_user)
			.await
			.map(|m| m.display_name().to_string())
			.unwrap_or_else(|_| retweeter_user.name.clone());

		let twitter_number = || {
			strings
				.tweet_esoteric_amount_prefix
				.pick_biased(Ratio::new(1, 5))
				.cloned()
				.or_else(|| twitter_random_number(strings))
		};

		let retweets = twitter_number();
		let quotes = twitter_number();
		let likes = twitter_number();

		Ok(TweetData {
			retweeter: strings
				.tweet_retweeter
				.pick_biased_or(Ratio::new(1, 2), &retweeter)
				.clone(),
			avatar: member.face(),
			name: member.display_name().to_string(),
			verified: verified_role.is_none()
				|| verified_role.is_some_and(|id| member.roles.iter().any(|&r| r == id)),
			at: member.user.name,
			tweet_text: content,
			hour: format!(
				"{:02}:{:02}",
				first.timestamp.hour(),
				first.timestamp.minute()
			),
			month: strings
				.tweet_month
				.pick_or(&first.timestamp.month().to_string())
				.clone(),
			day: format!("{:02}", first.timestamp.day()),
			year: format!("{:04}", first.timestamp.year()),

			client: strings.tweet_client.pick().clone(),
			any_numbers: retweets.is_some() || quotes.is_some() || likes.is_some(),
			retweets,
			quotes,
			likes,
			more_tweets: vec![],

			theme: Some(
				crate::helpers::handlebars::TWEET_THEME_GRAB_BAG
					.pick_biased(Ratio::new(2, 1))
					.clone(),
			),
			reply_to: first
				.referenced_message
				.as_ref()
				.map(|msg| msg.author.name.clone()),
			image: attachment,
			fact_check: strings.tweet_fact_check.pick().cloned(),
		})
	}

	async fn tweet_extra_data_from_message(
		&self,
		ctx: &Context,
		messages: Vec<Message>,
		verified_role: Option<u64>,
		reaction_time: Timestamp,
	) -> GovanResult<TweetMoreData> {
		let attachment = messages.iter().find_map(|msg| msg.any_image());

		let content = content_from_msgs(
			&messages,
			ctx,
			attachment.as_ref().unwrap_or(&String::new()),
		)
		.await?;

		let first = messages.first().unwrap();
		let time_diff = *reaction_time - *first.timestamp;

		let time_str = {
			if time_diff.whole_hours() >= 24 {
				format!(
					"{} {} {}",
					first.timestamp.day(),
					first.timestamp.month(),
					first.timestamp.year()
				)
			} else if time_diff.whole_hours() > 0 {
				format!("{}h", time_diff.whole_hours())
			} else if time_diff.whole_minutes() > 0 {
				format!("{}m", time_diff.whole_minutes())
			} else if time_diff.whole_seconds() > 0 {
				format!("{}s", time_diff.whole_seconds())
			} else {
				format!("{}ns", time_diff.whole_nanoseconds())
			}
		};

		let member = first.member(&ctx).await?;

		let strings = &self.data().await.strings;

		let twitter_number = || {
			strings
				.tweet_esoteric_amount_suffix
				.pick_biased(Ratio::new(1, 20))
				.cloned()
				.or_else(|| twitter_random_number(strings))
		};

		let replies = twitter_number().unwrap_or("".to_string());
		let retweets = twitter_number().unwrap_or("".to_string());
		let likes = twitter_number().unwrap_or("".to_string());

		Ok(TweetMoreData {
			avatar: member.face(),
			name: strings
				.tweet_username
				.pick_biased(Ratio::new(1, 5))
				.cloned()
				.unwrap_or(member.display_name().to_string()),
			verified: verified_role.is_some_and(|id| member.roles.iter().any(|&r| r == id)),
			at: member.user.name,
			time: strings
				.tweet_esoteric_time
				.pick_biased(Ratio::new(1, 5))
				.cloned()
				.unwrap_or(time_str),
			tweet_text: strings
				.tweet_extra_text
				.pick_biased(Ratio::new(1, 10))
				.cloned()
				.unwrap_or(content),
			replies,
			retweets,
			likes,
			reply_to: strings
				.tweet_extra_reply
				.pick_biased(Ratio::new(1, 10))
				.cloned()
				.or_else(|| {
					first
						.referenced_message
						.as_ref()
						.map(|msg| msg.author.name.clone())
				}),
			image: attachment,
		})
	}

	pub async fn maybe_retweet(
		&self,
		ctx: &Context,
		msg: &Message,
		reaction: &Reaction,
		with_context: bool,
		verified_role: Option<u64>,
	) -> GovanResult {
		let now = Timestamp::now();
		let screenshotter = self.screenshotter().await?;

		let channel = msg
			.channel(&ctx)
			.await?
			.guild()
			.ok_or_else(govanerror::debug_lazy!(
				log = "Trying to retweet in DMs",
				user = "You're not in a guild, silly!"
			))?;

		let messages = channel
			.messages(
				&ctx,
				GetMessages::default().after(msg.id.get() - 1).limit(50),
			)
			.await?;

		if messages.is_empty() {
			return Err(govanerror::error!(
			  log fmt = ("Message {} cannot be retweeted, it doesn't exist?", msg.id),
			  user = "I could not access the Infinitely Tall Cylinder Earth Twitter API. Please try again later"
			));
		}

		lazy_static::lazy_static! {
		  static ref MAX_TIME_DIFF: chrono::Duration = chrono::Duration::seconds(30);
		}

		let mut had_image = false;
		let mut author = UserId::new(1);
		let mut group = 0;
		let mut last_timestamp =
			DateTime::from_timestamp(msg.timestamp.unix_timestamp(), 0).unwrap();
		let context = messages
			.into_iter()
			.rev()
			.map(|mut msg| {
				msg.guild_id = Some(channel.guild_id);
				msg
			})
			.group_by(move |msg| {
				let timestamp =
					DateTime::from_timestamp(msg.timestamp.unix_timestamp(), 0).unwrap();
				if msg.author.id != author
					|| timestamp - last_timestamp > *MAX_TIME_DIFF
					|| msg.referenced_message.is_some()
				{
					had_image = false;
					author = msg.author.id;
					group += 1;
				}

				last_timestamp = timestamp;

				if msg.any_image().is_some() {
					if had_image {
						group += 1;
					} else {
						had_image = true;
					}
				}

				group
			})
			.into_iter()
			.take(10)
			.map(|i| i.1.collect_vec())
			.collect_vec();

		let first = context.first().ok_or_else(govanerror::error_lazy!(
		  log fmt = ("Message {} cannot be retweeted, it doesn't have available messages?", msg.id),
		  user = "I could not access the Infinitely Tall Cylinder Earth Twitter API. Please try again later"
		))?;

		let mut tweet_data = self
			.tweet_data_from_message(ctx, first, reaction, verified_role)
			.await?;

		let reactor = reaction.user(&ctx).await?;

		if with_context {
			tweet_data.more_tweets.extend(
				util::collect_async(context.into_iter().skip(1).map(|msgs| async {
					let id = msgs.first().map(|msg| msg.id.get()).unwrap_or(0);
					self.tweet_extra_data_from_message(ctx, msgs, verified_role, now)
						.await
						.ok_or_log(&format!("Error creating data from message {}", id))
				}))
				.await
				.into_iter()
				.flatten(),
			);
		}

		let data = screenshotter.twitter(tweet_data).await?;

		channel
			.send_message(
				&ctx,
				CreateMessage::default()
					.reference_message(msg)
					.allowed_mentions(CreateAllowedMentions::default().empty_users())
					.add_file(CreateAttachment::bytes(
						data,
						format!("tweet_by_{}.png", reactor),
					)),
			)
			.await?;

		Ok(())
	}
}
