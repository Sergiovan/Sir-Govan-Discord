use crate::prelude::*;
use std::convert::Infallible;

use crate::bot::Bot;

use crate::helpers::handlebars::{TweetData, TweetMoreData};

use itertools::Itertools;
use num_rational::Ratio;
use serenity::model::prelude::*;
use serenity::prelude::*;

use lazy_static::lazy_static;
use regex::Regex;

fn content_from_msgs(msgs: &[Message], ctx: &Context, filter: &str, guild_id: GuildId) -> String {
	use html_escape::encode_quoted_attribute as html_encode;

	lazy_static! {
		static ref DISCORD_TAG_SAVER: Regex =
			Regex::new(r"<(a?:(?P<NAME>.*?):|@&?|#)(?P<ID>[0-9]+)>").unwrap();
		static ref TAG_REVERSAL: Regex = Regex::new(r"[\x00|\x01|\x02]").unwrap();
	}

	fn linkify(s: &str) -> String {
		format!(r#"<span class="twitter-link">{}</span>"#, s)
	}

	let content = msgs
		.iter()
		.filter(|msg| filter != msg.content)
		.map(|msg| msg.content.clone())
		.collect_vec()
		.join("\n");

	let content = DISCORD_TAG_SAVER.replace_all(&content, |capture: &regex::Captures| {
		let typ = capture.get(1).unwrap().as_str();
		let id = capture.name("ID").unwrap().as_str();

		let typ = match typ {
			"@&" => "@\x02",
			_ => typ,
		};

		format!("\x00{}{}\x01", typ, id)
	});

	let content = html_encode(&content);
	let content = TAG_REVERSAL.replace_all(&content, |capture: &regex::Captures| {
		let byte = capture.get(0).unwrap().as_str();
		match byte {
			"\x00" => "<",
			"\x01" => ">",
			"\x02" => "&",
			_ => "",
		}
	});

	let content =
		data::regex::DISCORD_URL.replace_all(&content, r#"<span class="twitter-link">$0</span>"#);
	let content =
		data::regex::DISCORD_USER_AT.replace_all(&content, |capture: &regex::Captures| {
			let Ok(id) = capture.name("ID").unwrap().as_str().parse::<u64>() else {
        return capture.get(0).unwrap().as_str().to_string();
      };
			tokio::task::block_in_place(|| {
				tokio::runtime::Handle::current().block_on(async move {
					let Ok(user) = UserId(id).to_user(&ctx).await else {
          return capture.get(0).unwrap().as_str().to_string();
        };
					linkify(&format!("@{}", html_encode(&user.name)))
				})
			})
		});
	let content =
		data::regex::DISCORD_ROLE_AT.replace_all(&content, |capture: &regex::Captures| {
			let Ok(id) = capture.name("ID").unwrap().as_str().parse::<u64>() else {
        return capture.get(0).unwrap().as_str().to_string();
      };
			let Some(role) = ctx.cache.role(guild_id, id) else {
        return capture.get(0).unwrap().as_str().to_string();
      };
			linkify(&format!("@{}", html_encode(&role.name)))
		});
	let content =
		data::regex::DISCORD_CHANNEL_AT.replace_all(&content, |capture: &regex::Captures| {
			let Ok(id) = capture.name("ID").unwrap().as_str().parse::<u64>() else {
        return capture.get(0).unwrap().as_str().to_string();
      };
			tokio::task::block_in_place(|| {
				tokio::runtime::Handle::current().block_on(async move {
					let Ok(channel) = ChannelId(id).to_channel(&ctx).await else {
            return capture.get(0).unwrap().as_str().to_string();
          };
					let Some(channel) = channel.guild() else {
            return capture.get(0).unwrap().as_str().to_string();
          };
					if channel.guild_id != guild_id {
						return capture.get(0).unwrap().as_str().to_string();
					}
					linkify(&format!("#{}", html_encode(&channel.name)))
				})
			})
		});
	let content =
		data::regex::DISCORD_EMOJI_REGEX.replace_all(&content, |capture: &regex::Captures| {
			let Ok(id) = capture.name("ID").unwrap().as_str().parse::<u64>() else {
          return capture.get(0).unwrap().as_str().to_string();
        };
			let animated = capture.name("ANIMATED").is_some_and(|g| !g.is_empty());
			format!(
				r#"<img class="emoji" src="{}">"#,
				util::url_from_discord_emoji(id, animated)
			)
		});
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

	content.replace('\n', "<br>")
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
	) -> Option<TweetData> {
		let guild_id = reaction.guild_id.log_if_none(&format!(
			"Reaction {:?} on message {} was not in a guild",
			reaction.emoji, reaction.message_id
		))?;

		let attachment = messages.iter().find_map(|msg| {
			msg.attachments.first().map(|a| a.url.clone()).or(msg
				.embeds
				.first()
				.and_then(|e| e.image.as_ref().map(|i| i.url.clone()).or(e.url.clone())))
		});

		let content = content_from_msgs(
			messages,
			ctx,
			attachment.as_ref().unwrap_or(&String::new()),
			guild_id,
		);

		println!("{}", content);

		let first = messages.first().unwrap();

		let member = first.member(&ctx).await.ok_or_log(&format!(
			"Could not get member data from {}",
			first.author.id
		))?;

		let strings = &self.data.read().await.strings;
		let retweeter_user = reaction.user(&ctx).await.ok_or_log(&format!(
			"Could not get user for reaction on {}",
			reaction.message_id
		))?;
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

		Some(TweetData {
			retweeter: strings
				.tweet_retweeter
				.pick_biased_or(Ratio::new(1, 2), &retweeter)
				.clone(),
			avatar: member.face(),
			name: member.display_name().into_owned(),
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

			client: strings.tweet_client.pick().unwrap().clone(),
			any_numbers: retweets.is_some() || quotes.is_some() || likes.is_some(),
			retweets,
			quotes,
			likes,
			more_tweets: vec![],

			theme: crate::helpers::handlebars::TWEET_THEME_GRAB_BAG
				.pick_biased(Ratio::new(2, 1))
				.cloned(),
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
		reaction: &Reaction,
		verified_role: Option<u64>,
		original_timestamp: Timestamp,
	) -> Option<TweetMoreData> {
		let guild_id = reaction.guild_id.log_if_none(&format!(
			"Reaction {:?} on message {} was not in a guild",
			reaction.emoji, reaction.message_id
		))?;

		let attachment = messages.iter().find_map(|msg| {
			msg.attachments.first().map(|a| a.url.clone()).or(msg
				.embeds
				.first()
				.and_then(|e| e.image.as_ref().map(|i| i.url.clone())))
		});

		let content = content_from_msgs(
			&messages,
			ctx,
			attachment.as_ref().unwrap_or(&String::new()),
			guild_id,
		);

		let first = messages.first().unwrap();
		let time_diff = *first.timestamp - *original_timestamp;

		let time_str = {
			if time_diff.whole_hours() >= 24 {
				format!(
					"{} {} {}",
					first.timestamp.day(),
					first.timestamp.month() as u32,
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

		let member = first.member(&ctx).await.ok_or_log(&format!(
			"Could not get member data from {}",
			first.author.id
		))?;

		let strings = &self.data.read().await.strings;

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

		Some(TweetMoreData {
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
		ctx: Context,
		msg: Message,
		reaction: Reaction,
		with_context: bool,
		verified_role: Option<u64>,
	) -> Option<Infallible> {
		let screenshotter = self.get_screenshotter().await;
		let screenshotter = screenshotter.as_ref();

		let Some(screenshotter) = screenshotter else {
      msg.reply_report(&ctx, "I could not connect to the Infinitely Tall Cylinder Earth Twitter servers. Please try again later")
        .await;
      return None;
    };

		let channel = msg
			.channel(&ctx)
			.await
			.ok_or_log("Could not fetch message channels")?
			.guild()
			.log_if_none("Message was not in guild")?;

		let messages = channel
			.messages(&ctx, |b| b.after(msg.id.0 - 1).limit(50))
			.await
			.ok_or_log("Contextual messages could not be fetched")?;

		if messages.is_empty() {
			msg.reply_report(&ctx, "I could not access the Infinitely Tall Cylinder Earth Twitter API. Please try again later")
      .await;
			return None;
		}

		let context = messages
			.into_iter()
			.rev()
			.map(|mut msg| {
				msg.guild_id = Some(channel.guild_id);
				msg
			})
			.group_by(|e| e.author.id)
			.into_iter()
			.map(|i| i.1.collect_vec())
			.collect_vec();

		let first = context.first().log_if_none("No messages found")?;

		let tweet_data = self
			.tweet_data_from_message(&ctx, first, &reaction, verified_role)
			.await
			.log_if_none("Error creating data from message");

		if tweet_data.is_none() {
			msg.reply_report(
				&ctx,
				"I was not allowed to gather data for your tweet. Please try again later",
			)
			.await;
			return None;
		}

		let reactor = reaction
			.user(&ctx)
			.await
			.ok_or_log("Could not get reactor")?;

		let mut tweet_data = tweet_data.unwrap();

		if with_context {
			tweet_data.more_tweets.extend(
				futures::future::join_all(context.into_iter().skip(1).map(|msgs| async {
					let id = msgs.first().map(|msg| msg.id.0).unwrap_or(0);
					self.tweet_extra_data_from_message(
						&ctx,
						msgs,
						&reaction,
						verified_role,
						msg.timestamp,
					)
					.await
					.log_if_none(&format!("Error creating data from message {}", id))
				}))
				.await
				.into_iter()
				.flatten(),
			);
		}

		let data = match screenshotter.twitter(tweet_data).await {
			Ok(data) => data,
			Err(e) => {
				logger::error(&format!("Could not screenshot twitter: {}", e));
				msg.reply_report(&ctx, "My camera broke. Sorry about that. But your tweet reached the Infinitely Tall Cylinder Earth at least").await;
				return None;
			}
		};

		channel
			.send_message(&ctx, |b| {
				b.reference_message(&msg)
					.allowed_mentions(|b| b.empty_users())
					.add_file(AttachmentType::Bytes {
						data: std::borrow::Cow::Borrowed(data.as_slice()),
						filename: format!("tweet_by_{}.png", reactor),
					})
			})
			.await
			.ok_or_log(&format!("Could not send message to {}", channel.id));

		None
	}
}
