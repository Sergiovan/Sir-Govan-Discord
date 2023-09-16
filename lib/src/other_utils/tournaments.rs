use crate::args::*;
use crate::util::random;

use anyhow::{anyhow, bail};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::builder::{
	CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage, EditMessage, GetMessages,
};
use serenity::model::prelude::*;
use serenity::prelude::*;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug)]
struct TournamentData {
	post_channel: ChannelId,
	entries: Vec<TournamentDataEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TournamentDataEntry {
	title: String,
	comment: String,
	ignore: bool,

	pin_message: MessageId,
	pin_channel: ChannelId,

	original_message: MessageId,
	original_channel: ChannelId,

	pins: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Entry {
	entry: u64,
	votes: u64,
}

impl Entry {
	fn reset(mut self) -> Self {
		self.votes = 0;
		self
	}
}

#[derive(Serialize, Deserialize, Debug)]
struct Battle {
	a: Entry,
	b: Option<Entry>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Round {
	totals: HashMap<String, u64>,
	battles: Vec<Battle>,
}

struct DismantledEmbed {
	color: Color,
	author_name: String,
	author_avatar: String,

	message: MessageId,
	channel: ChannelId,

	content: String,
	image: Option<String>,
}

const TOURNAMENT_DIR: &str = "res/tournaments";
const TOURNAMENT_FILE: &str = "tournament.toml";

const A_EMOJI: &str = "🅰️";
const A_URL: &str = "https://twemoji.maxcdn.com/v/latest/72x72/1f170.png";
const B_EMOJI: &str = "🅱️";
const B_URL: &str = "https://twemoji.maxcdn.com/v/latest/72x72/1f171.png";

lazy_static! {
	pub static ref FOOTER_PIN: Regex = Regex::new(r"(\d+) - (\d+)").unwrap();
	pub static ref CONTENT_TELEPORT: Regex = Regex::new(r"\[Click to teleport\]\(.*?\)").unwrap();
  pub static ref FOOTER_TOURNAMENT: Regex = Regex::new(r"(?P<tournament>.*) tournament \| Round (?P<round>\d+) \| Battle (?P<battle>\d+) \| Entry (?P<entry>A|B)").unwrap();
}

async fn run_command(ctx: &Context, args: &TournamentArgs) -> anyhow::Result<()> {
	use std::fs;

	match &args.command {
		TournamentCommand::Create(args) => {
			let tournament_path = tournament_dir(&args.tournament_name);

			if tournament_path.exists() {
				bail!(
					"Tournament {} already exists!: {}",
					args.tournament_name,
					tournament_path.display()
				);
			}

			fs::create_dir_all(tournament_path)?;

			let tournament_data_path = tournament_data(&args.tournament_name);

			if tournament_data_path.exists() {
				bail!(
					"How is this even fucking possible??? {} already exists",
					tournament_data_path.display()
				);
			}

			let channel = ChannelId::new(args.pin_channel).to_channel(&ctx).await?;
			let channel = channel.guild().ok_or(anyhow!(
				"Channel {} is not a guild channel",
				args.pin_channel
			))?;

			let msgs = fetch_messages(
				ctx,
				channel,
				MessageId::new(args.message_first),
				MessageId::new(args.message_last),
			)
			.await?;

			let reaction_type = if let Some(groups) =
				crate::data::regex::DISCORD_EMOJI_REGEX.captures(&args.pin_emoji)
			{
				ReactionType::Custom {
					animated: groups.name("ANIMATED").is_some_and(|m| !m.is_empty()),
					id: EmojiId::new(groups.name("ID").unwrap().as_str().parse::<u64>()?),
					name: Some(groups.name("NAME").unwrap().as_str().to_string()),
				}
			} else {
				ReactionType::Unicode(args.pin_emoji.clone())
			};

			let reaction_type = &reaction_type;

			let mut entries = vec![];

			// For the future: Do not try to use `join_all` or any other sort of batching, the
			// serenity Discord library does not enjoy that at all
			for msg in msgs.into_iter() {
				entries.push(create_entry(ctx, &msg, dismantle_embed(&msg)?, reaction_type).await?);
			}

			let data = TournamentData {
				post_channel: ChannelId::new(args.tournament_channel),
				entries,
			};

			let data = toml::ser::to_string(&data)?;

			fs::write(tournament_data_path, data)?;

			let round0_path = round_data(&args.tournament_name, 0);
			if round0_path.exists() {
				fs::remove_file(round0_path)?;
			}
		}
		TournamentCommand::Verify(args) => {
			let tournament_name = &args.tournament_name;

			let round0_file_path = round_data(tournament_name, 0);
			let tournament_data_path = tournament_data(tournament_name);

			if round0_file_path.exists() {
				bail!("{} already exists", round0_file_path.display());
			}

			let tournament_data: TournamentData =
				toml::from_str(&fs::read_to_string(&tournament_data_path)?)?;

			let mut entries = (0..tournament_data.entries.len() as u64)
				.filter(|e| !tournament_data.entries[*e as usize].ignore)
				.collect_vec();

			if entries.is_empty() {
				bail!(
					"Tournament {} from file {} has no entries",
					tournament_name,
					tournament_data_path.display()
				);
			}

			shuffle(&mut entries);

			let mut round = Round::default();

			for mut chunk in &entries.into_iter().chunks(2) {
				let a = chunk.next().unwrap();
				let b = chunk.next();

				round.battles.push(Battle {
					a: Entry { entry: a, votes: 0 },
					b: b.map(|e| Entry { entry: e, votes: 0 }),
				});

				round.totals.insert(a.to_string(), 0);
				if let Some(b) = b {
					round.totals.insert(b.to_string(), 0);
				}
			}

			fs::write(&round0_file_path, toml::to_string(&round)?)?;
		}
		TournamentCommand::PostRound(args) => {
			let tournament_name = &args.tournament_name;

			let round_data_path = round_data(tournament_name, args.round);

			if round_data_path.exists() {
				bail!(
					"Round file {} for tournament {} already exists",
					round_data_path.display(),
					tournament_name
				);
			}

			let tournament_data: TournamentData =
				toml::from_str(&fs::read_to_string(tournament_data(tournament_name))?)?;

			let tournament_channel = tournament_data
				.post_channel
				.to_channel(&ctx)
				.await?
				.guild()
				.ok_or(anyhow!(
					"Channel {} is not a guild channel",
					tournament_data.post_channel
				))?;

			let previous_round: Round = toml::from_str(&fs::read_to_string(round_data(
				tournament_name,
				args.round - 1,
			))?)?;

			let current_round = if args.round - 1 == 0 {
				previous_round
			} else {
				let mut totals = HashMap::<u64, u64>::default();

				let winners = previous_round
					.battles
					.iter()
					.map(|b| {
						let winner = get_winner(b, &previous_round, &tournament_data);
						totals.insert(
							winner.entry,
							previous_round
								.totals
								.get(&winner.entry.to_string())
								.unwrap_or(&0) + winner.votes,
						);

						winner
					})
					.collect_vec();

				let winners = if args.reduce {
					reduce_winners(&tournament_data, &previous_round, &winners)
				} else {
					winners
				};

				let battles = winners
					.into_iter()
					.chunks(2)
					.into_iter()
					.map(|mut c| {
						let a = c.next().unwrap();
						let b = c.next();

						Battle {
							a: Entry {
								entry: a.entry,
								votes: 0,
							},
							b: b.map(|e| Entry {
								entry: e.entry,
								votes: 0,
							}),
						}
					})
					.collect_vec();

				Round {
					totals: totals
						.into_iter()
						.map(|(k, v)| (k.to_string(), v))
						.collect(),
					battles,
				}
			};

			let odd = current_round.battles.last().is_some_and(|b| b.b.is_none());

			let msg_create_futures = current_round.battles.iter().enumerate().map(|(i, b)| {
				create_battle(
					ctx,
					tournament_name,
					args.round,
					b,
					i as u64,
					&tournament_data,
				)
			});

			let mut msg_creates = Vec::with_capacity(current_round.battles.len());

			for msg_create in msg_create_futures {
				msg_creates.push(msg_create.await?);
			}

			shuffle(&mut msg_creates);

			let mut start = tournament_channel
				.send_message(&ctx, CreateMessage::new().content(":)"))
				.await?;

			let mut msgs = Vec::with_capacity(msg_creates.len());

			for (i, msg_create) in msg_creates.into_iter().enumerate() {
				msgs.push(
					tournament_channel
						.send_message(&ctx, msg_create.content(format!("Match #{}", i + 1)))
						.await?,
				);
			}

			let end = tournament_channel
				.send_message(
					&ctx,
					CreateMessage::new().content(format!(
					"This is the end of the entries for round __#{}__ of the **{}** tournbament.\n\
					 To go back to the start of the round hitch a ride on {}",
					args.round,
					tournament_name,
					start.link()
				)),
				)
				.await?;

			for msg in msgs.iter() {
				msg.react(&ctx, ReactionType::Unicode(A_EMOJI.to_string()))
					.await?;
				msg.react(&ctx, ReactionType::Unicode(B_EMOJI.to_string()))
					.await?;
			}

			start
				.edit(
					&ctx,
					EditMessage::new().content(format!(
						"Round __#{}__ of the **{}** tournament.\n\
						 {} matches, with {} entries.\n\n\
						 To go to the end of this round take this elevator: {}",
						args.round,
						tournament_name,
						msgs.len(),
						msgs.len() * 2 - if odd { 1 } else { 0 },
						end.link()
					)),
				)
				.await?;

			fs::write(round_data_path, toml::to_string(&current_round)?)?;
		}
		TournamentCommand::VerifyRound(args) => {
			let tournament_name = &args.tournament_name;
			let tournament_data: TournamentData =
				toml::from_str(&fs::read_to_string(tournament_data(tournament_name))?)?;

			let tournament_channel = tournament_data
				.post_channel
				.to_channel(&ctx)
				.await?
				.guild()
				.ok_or(anyhow!(
					"{} is not a guild channel",
					tournament_data.post_channel
				))?;

			let to_check =
				find_round_messages(ctx, &tournament_channel, tournament_name, args.round).await?;

			let total_len = to_check.len() * 2;
			let mut missing = 0;
			let mut fixed = 0;

			for msg in to_check.into_iter() {
				let fix = |emoji: String| async {
					if let Err(e) = msg.react(&ctx, ReactionType::Unicode(emoji)).await {
						println!(
							"Error while {} reacting to {}: {:?}",
							A_EMOJI,
							msg.link(),
							e
						);
						false
					} else {
						true
					}
				};

				let a_reactions = msg
					.reactions
					.iter()
					.find(|r| r.reaction_type.unicode_eq(A_EMOJI));
				let b_reactions = msg
					.reactions
					.iter()
					.find(|r| r.reaction_type.unicode_eq(B_EMOJI));

				if a_reactions.is_none() || b_reactions.is_none() {
					if a_reactions.is_none() {
						println!(
							"Message {} is is missing its {} reaction",
							msg.link(),
							A_EMOJI
						);
						missing += 1;

						if args.fix && fix(A_EMOJI.to_string()).await {
							fixed += 1;
						}
					}

					if b_reactions.is_none() {
						println!(
							"Message {} is is missing its {} reaction",
							msg.link(),
							B_EMOJI
						);

						missing += 1;

						if args.fix && fix(B_EMOJI.to_string()).await {
							fixed += 1;
						}
					}

					continue;
				};

				let a_reactions = a_reactions.unwrap();
				let b_reactions = b_reactions.unwrap();

				if !a_reactions.me {
					missing += 1;
					println!("I did not {} react to message {}", A_EMOJI, msg.link());

					if args.fix && fix(A_EMOJI.to_string()).await {
						fixed += 1;
					}
				}
				if !b_reactions.me {
					missing += 1;
					println!("I did not {} react to message {}", B_EMOJI, msg.link());

					if args.fix && fix(B_EMOJI.to_string()).await {
						fixed += 1;
					}
				}
			}

			println!("Missed {}/{} reactions", missing, total_len);
			if args.fix {
				println!("Fixed {}/{} reactions", fixed, missing);
			}
		}
		TournamentCommand::FinishRound(args) => {
			let tournament_name = &args.tournament_name;
			let tournament_data: TournamentData =
				toml::from_str(&fs::read_to_string(tournament_data(tournament_name))?)?;

			let tournament_channel = tournament_data
				.post_channel
				.to_channel(&ctx)
				.await?
				.guild()
				.ok_or(anyhow!(
					"{} is not a guild channel",
					tournament_data.post_channel
				))?;

			let round_data_path = round_data(tournament_name, args.round);
			let mut round_data: Round = toml::from_str(&fs::read_to_string(&round_data_path)?)?;

			let to_decide =
				find_round_messages(ctx, &tournament_channel, tournament_name, args.round).await?;

			round_data.battles = to_decide
				.into_iter()
				.enumerate()
				.map(|(i, msg)| {
					if false {
						bail!("");
					}

					let battle = &round_data.battles[i];
					let a_votes = msg
						.reactions
						.iter()
						.find_map(|r| {
							if r.reaction_type.unicode_eq(A_EMOJI) {
								Some(r.count - 1)
							} else {
								None
							}
						})
						.ok_or(anyhow!(
							"Message {} was missing its {} reactions",
							msg.link(),
							A_EMOJI
						))?;
					let b_votes = msg
						.reactions
						.iter()
						.find_map(|r| {
							if r.reaction_type.unicode_eq(B_EMOJI) {
								Some(r.count - 1)
							} else {
								None
							}
						})
						.ok_or(anyhow!(
							"Message {} was missing its {} reactions",
							msg.link(),
							B_EMOJI
						))?;

					Ok(Battle {
						a: Entry {
							entry: battle.a.entry,
							votes: a_votes,
						},
						b: battle.b.as_ref().map(|b| Entry {
							entry: b.entry,
							votes: b_votes,
						}),
					})
				})
				.try_collect()?;

			fs::write(round_data_path, toml::to_string(&round_data)?)?;
		}
		TournamentCommand::CleanRound(args) => {
			let tournament_name = &args.tournament_name;
			let tournament_data: TournamentData =
				toml::from_str(&fs::read_to_string(tournament_data(tournament_name))?)?;

			let tournament_channel = tournament_data
				.post_channel
				.to_channel(&ctx)
				.await?
				.guild()
				.ok_or(anyhow!(
					"{} is not a guild channel",
					tournament_data.post_channel
				))?;

			let to_delete =
				find_round_messages(ctx, &tournament_channel, tournament_name, args.round).await?;

			let to_delete = to_delete.into_iter().map(|m| m.id).chunks(100);
			let to_delete = to_delete.into_iter().map(|c| c.collect_vec()).collect_vec();

			for batch in to_delete {
				tournament_channel.delete_messages(&ctx, batch).await?;
			}
		}
		TournamentCommand::Finish(args) => {}
	}

	Ok(())
}

async fn fetch_messages(
	ctx: &Context,
	channel: GuildChannel,
	after: MessageId,
	until: MessageId,
) -> anyhow::Result<Vec<Message>> {
	let after = MessageId::new(after.get() - 1);
	let until = MessageId::new(until.get() + 1);

	println!(
		"Fetching messages from channel {}: {} to {}",
		channel.name, after, until
	);

	let mut res = Vec::with_capacity(100);
	let mut after = after;

	loop {
		let mut batch = channel
			.messages(&ctx, GetMessages::default().after(after).limit(100))
			.await?
			.into_iter()
			.filter(|m| m.id < until)
			.rev()
			.collect::<Vec<_>>();

		// println!("{:?}", batch);

		if batch.is_empty() {
			break;
		}

		after = batch.last().unwrap().id;
		res.append(&mut batch);

		print!("\r{}", res.len());
	}

	println!("\rDone");

	Ok(res)
}

fn dismantle_embed(message: &Message) -> anyhow::Result<DismantledEmbed> {
	println!("Dismantling {}", message.id);

	let Some(embed) = message.embeds.first() else {
		bail!("Message {} had no embeds: {:?}", message.id, message)
	};

	if embed.author.is_none() || embed.author.as_ref().is_some_and(|e| e.icon_url.is_none()) {
		bail!(
			"Embed from {} does not have a proper author: {:?}",
			message.id,
			embed
		);
	}

	if embed.footer.is_none() || embed.footer.as_ref().is_some_and(|f| f.text.is_empty()) {
		bail!(
			"Embed from {} does not have a proper footer: {:?}",
			message.id,
			embed.footer
		);
	}

	if embed.description.is_none() && embed.fields.is_empty() {
		bail!(
			"Embed from {} has no valid description: {:?} and {:?}",
			message.id,
			embed.description,
			embed.fields
		);
	}

	let color = embed.colour.unwrap_or(Color::new(0xC0FFEE));

	let author = embed.author.as_ref().unwrap();
	let author_name = author.name.clone();
	let author_avatar = author.icon_url.as_ref().unwrap().clone();

	let footer_text = embed.footer.as_ref().unwrap().text.clone();
	let footer_groups = FOOTER_PIN.captures(&footer_text);

	if footer_groups.is_none() {
		bail!(
			"Footer from {} does not have the proper format: {}",
			message.id,
			footer_text
		);
	}

	let footer_groups = footer_groups.unwrap();
	let original_msg = footer_groups.get(1).unwrap().as_str().parse::<u64>()?;
	let original_channel = footer_groups.get(2).unwrap().as_str().parse::<u64>()?;

	let content = embed
		.description
		.as_ref()
		.unwrap_or_else(|| &embed.fields[0].value);

	if content.is_empty() {
		bail!("No content for embed of {}", message.id);
	}

	let image = embed
		.image
		.as_ref()
		.map(|i| &i.url)
		.or(embed.thumbnail.as_ref().map(|t| &t.url));

	Ok(DismantledEmbed {
		color,
		author_name,
		author_avatar,
		message: MessageId::new(original_msg),
		channel: ChannelId::new(original_channel),
		content: content.clone(),
		image: image.cloned(),
	})
}

async fn create_entry(
	ctx: &Context,
	msg: &Message,
	embed: DismantledEmbed,
	emoji: &ReactionType,
) -> anyhow::Result<TournamentDataEntry> {
	println!("Creating entry for {}", msg.id);

	let with_pins = |pins| TournamentDataEntry {
		title: "".to_string(),
		comment: format!(
			" {} {} ",
			embed.content,
			embed.image.unwrap_or(String::new())
		),
		ignore: false,

		pin_channel: msg.channel_id,
		pin_message: msg.id,

		original_channel: embed.channel,
		original_message: embed.message,

		pins,
	};

	println!("Fetching channel {}", embed.channel);

	let channel: Channel = embed.channel.to_channel(&ctx).await?;
	let channel = match channel {
		Channel::Guild(channel) => channel,
		_ => bail!("Wrong sort of channel: {:?}", channel),
	};

	println!("Fetching message {} from {}", embed.message, channel.name);
	let message = channel.message(&ctx, embed.message).await;

	let message = match message {
		Ok(message) => message,
		Err(e) => {
			println!(
				"Channel {} has no message with id {}. Might have been deleted. Assuming 4 pins: {}",
				channel.name, embed.message, e
			);
			return Ok(with_pins(4));
		}
	};

	println!("Checking reactions on {}", embed.message);
	let reaction = message.reactions.iter().find(|r| &r.reaction_type == emoji);

	if reaction.is_none() {
		println!(
			"Message {} has no reactions of type {}. Assuming 4",
			message.id, emoji
		);
		return Ok(with_pins(4));
	}

	println!("Done with {}", msg.id);

	Ok(with_pins(reaction.unwrap().count))
}

async fn create_battle(
	ctx: &Context,
	tournament_name: &str,
	round_nr: u64,
	battle: &Battle,
	battle_nr: u64,
	data: &TournamentData,
) -> anyhow::Result<CreateMessage> {
	let create_embed = |entry_nr: u64, is_a: bool| async move {
		let name = if is_a { A_EMOJI } else { B_EMOJI };

		let footer = format!(
			"{} tournament | Round {} | Battle {} | Entry {}",
			tournament_name,
			round_nr,
			battle_nr + 1,
			if is_a { 'A' } else { 'B' }
		);

		println!("Creating embed for {}", footer);
		if entry_nr >= data.entries.len() as u64 {
			bail!("Battle {} ({}) does not exist", name, entry_nr);
		}

		let entry = &data.entries[entry_nr as usize];

		let channel = entry.pin_channel.to_channel(&ctx).await?;
		let channel = match channel {
			Channel::Guild(channel) => channel,
			_ => bail!(
				"Wrong sort of channel in battle {} ({}): {:?}",
				name,
				entry_nr,
				channel
			),
		};

		let message = channel.message(&ctx, entry.pin_message).await?;

		let original_channel = entry.original_channel.to_channel(&ctx).await?;
		let original_channel = match original_channel {
			Channel::Guild(channel) => channel,
			_ => bail!(
				"Wrong sort of original channel in battle {} ({}): {:?}",
				name,
				entry_nr,
				original_channel
			),
		};

		let original_message = original_channel
			.message(&ctx, entry.original_message)
			.await?;
		let dismantled_embed = dismantle_embed(&message)?;

		let content = CONTENT_TELEPORT.replace(&dismantled_embed.content, "");

		let original_message_url = original_message.link_ensured(&ctx).await;

		let mut embed = CreateEmbed::default();

		embed = embed
			.title(entry.title.clone())
			.color(dismantled_embed.color)
			.url(original_message_url)
			.author(
				CreateEmbedAuthor::new(original_message.author.name.clone()).icon_url(
					original_message
						.author
						.avatar_url()
						.unwrap_or(original_message.author.default_avatar_url()),
				),
			)
			.thumbnail(if is_a { A_URL } else { B_URL })
			.description(content)
			.footer(CreateEmbedFooter::new(footer));

		if let Some(image) = dismantled_embed.image {
			embed = embed.image(image);
		}

		Ok(embed)
	};

	if battle.b.is_none() {
		let embeds = vec![create_embed(battle.a.entry, true).await?];
		Ok(CreateMessage::default().add_embeds(embeds))
	} else {
		let futures = vec![
			create_embed(battle.a.entry, true),
			create_embed(battle.b.as_ref().unwrap().entry, false),
		]
		.into_iter();
		let embeds = futures::future::try_join_all(futures).await?;

		Ok(CreateMessage::default().add_embeds(embeds))
	}
}

async fn find_round_messages(
	ctx: &Context,
	tournament_channel: &GuildChannel,
	tournament_name: &str,
	round_nr: u64,
) -> anyhow::Result<Vec<Message>> {
	let msgs = tournament_channel
		.messages(&ctx, GetMessages::default().limit(1))
		.await?;
	let mut last = msgs
		.first()
		.ok_or(anyhow!(
			"No messages fetched from {}",
			tournament_channel.name
		))?
		.id;

	let mut res = vec![];

	loop {
		let msgs = tournament_channel
			.messages(&ctx, GetMessages::default().before(last).limit(100))
			.await?;

		last = msgs.last().expect("No more messages").id;
		let myself = ctx.cache.current_user().id;

		let mut battles = msgs
			.into_iter()
			.filter(|m| m.author.id == myself && !m.embeds.is_empty())
			.filter_map(|m| {
				let embed = m.embeds.first()?;
				let footer = embed.footer.as_ref()?;
				let captures = FOOTER_TOURNAMENT.captures(&footer.text)?;
				if captures
					.name("tournament")
					.is_some_and(|m| m.as_str() == tournament_name)
					&& captures
						.name("round")
						.is_some_and(|m| m.as_str() == round_nr.to_string())
				{
					captures
						.name("battle")
						.and_then(|c| c.as_str().parse::<u64>().ok())
						.map(|n| (m, n))
				} else {
					None
				}
			})
			.collect_vec();

		if battles.is_empty() {
			break;
		} else {
			res.append(&mut battles);
		}
	}

	let res = res
		.into_iter()
		.sorted_by(|(_, a), (_, b)| a.cmp(b))
		.map(|(m, _)| m)
		.collect_vec();

	Ok(res)
}

fn reduce_winners<'a>(
	data: &TournamentData,
	previous_round: &Round,
	winners: &[&'a Entry],
) -> Vec<&'a Entry> {
	if winners.len() <= 1 {
		return winners.into();
	}

	let reduced_exponent = (winners.len() as f64).log2().floor() as usize;
	let final_amount = 2_usize.pow(reduced_exponent as u32);

	if final_amount == winners.len() {
		return winners.into();
	}

	let death_set = winners
		.iter()
		.sorted_by(|a, b| get_best_entry(a, b, previous_round, data))
		.skip(final_amount)
		.map(|e| e.entry)
		.collect::<HashSet<_>>();

	winners
		.iter()
		.filter(|w| !death_set.contains(&w.entry))
		.copied()
		.collect_vec()
}

fn get_winner_ord(
	battle: &Battle,
	previous_round: &Round,
	tournament_data: &TournamentData,
) -> Ordering {
	let a = &battle.a;
	let b = &battle.b;

	if b.is_none() {
		return Ordering::Less;
	}

	let b = b.as_ref().unwrap();

	get_best_entry(a, b, previous_round, tournament_data)
}

fn get_winner<'a>(
	battle: &'a Battle,
	previous_round: &Round,
	tournament_data: &TournamentData,
) -> &'a Entry {
	if get_winner_ord(battle, previous_round, tournament_data) == Ordering::Less {
		&battle.a
	} else {
		battle.b.as_ref().unwrap()
	}
}

fn get_best_entry<'a>(
	a: &'a Entry,
	b: &'a Entry,
	previous_round: &Round,
	tournament_data: &TournamentData,
) -> Ordering {
	if a.votes > b.votes {
		Ordering::Less
	} else if b.votes > a.votes {
		Ordering::Greater
	} else if tournament_data.entries[a.entry as usize].pins
		> tournament_data.entries[b.entry as usize].pins
	{
		Ordering::Less
	} else if tournament_data.entries[b.entry as usize].pins
		> tournament_data.entries[a.entry as usize].pins
	{
		Ordering::Greater
	} else if previous_round.totals[&a.entry.to_string()]
		> previous_round.totals[&b.entry.to_string()]
	{
		Ordering::Less
	} else if previous_round.totals[&b.entry.to_string()]
		> previous_round.totals[&a.entry.to_string()]
	{
		Ordering::Greater
	} else if a.entry < b.entry {
		Ordering::Less
	} else {
		Ordering::Greater
	}
}

fn tournament_dir(tournament_name: &str) -> PathBuf {
	Path::new(TOURNAMENT_DIR).join(tournament_name)
}

fn tournament_data(tournament_name: &str) -> PathBuf {
	tournament_dir(tournament_name).join(TOURNAMENT_FILE)
}

fn round_data(tournament_name: &str, round_nr: u64) -> PathBuf {
	tournament_dir(tournament_name).join(format!("round_{}.toml", round_nr))
}

fn shuffle<T>(vec: &mut Vec<T>) {
	for i in (1..(vec.len())).rev() {
		let j = random::from_range(0..=i);
		vec.swap(i, j);
	}
}

// Setup below this point

use async_trait::async_trait;
use serenity::gateway::ShardManager;
use std::sync::Arc;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
	type Value = Arc<ShardManager>;
}

pub struct BotEventHandler {
	args: TournamentArgs,
}

#[async_trait]
impl EventHandler for BotEventHandler {
	async fn ready(&self, ctx: Context, _: Ready) {
		ctx.cache.set_max_messages(10000);

		if let Err(e) = run_command(&ctx, &self.args).await {
			println!("Error while running tournament: {:?}", e);
		}

		ctx.data
			.read()
			.await
			.get::<ShardManagerContainer>()
			.unwrap()
			.shutdown_all()
			.await;
	}
}

pub async fn tournament(token: &str, args: TournamentArgs) {
	let intents = GatewayIntents::GUILDS
		| GatewayIntents::GUILD_MESSAGES
		| GatewayIntents::MESSAGE_CONTENT
		| GatewayIntents::GUILD_MESSAGE_REACTIONS;

	let mut client = Client::builder(token, intents)
		.event_handler(BotEventHandler { args })
		.await
		.expect("Err creating client");

	{
		let mut data = client.data.write().await;
		data.insert::<ShardManagerContainer>(client.shard_manager.clone());
	}

	let shard_manager = client.shard_manager.clone();
	{
		tokio::spawn(async move {
			tokio::signal::ctrl_c()
				.await
				.expect("Could not register Ctrl+C handler");
			print!("\r");
			shard_manager.shutdown_all().await;
		});
	}

	if let Err(why) = client.start().await {
		println!("Client error: {:?}", why);
	}
}
