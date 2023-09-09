use std::num::ParseIntError;

use crate::prelude::*;

use crate::bot::Bot;
use serenity::model::prelude::*;
use serenity::prelude::*;

use super::commander::Arguments;

use sirgovan_macros::command;

use rand::Rng;

#[derive(thiserror::Error, Debug)]
enum ColorError {
	#[error("")]
	NotInGuild,
	#[error("Could not get member from {0}: {1}")]
	MemberFailure(UserId, #[source] anyhow::Error),
	#[error("")]
	ParseIntError(#[from] ParseIntError),
	#[error("")]
	HexTooLarge,
	#[error("Could not change role {1} for {2} to color #{3:06X}: {0}")]
	RoleEditError(#[source] serenity::Error, RoleId, UserId, u64),
}

impl Reportable for ColorError {
	fn to_user(&self) -> Option<String> {
		match self {
			Self::MemberFailure(..) => {
				Some("The Discord API is being funny, please try again later".into())
			}
			Self::ParseIntError(..) => Some("That is an invalid hex number".into()),
			Self::HexTooLarge => Some("That hex is too large".into()),
			Self::RoleEditError(..) => {
				Some("Something went wrong. Could not change your role color".into())
			}
			_ => None,
		}
	}
}

#[command(aliases = ["colour"])]
async fn color<'a>(
	&self,
	ctx: &Context,
	msg: &'a Message,
	mut words: Arguments<'a>,
	_bot: &Bot,
) -> GovanResult {
	msg.guild_id.ok_or_else(govanerror::debug_lazy!(
		log = "Command used outside of guild",
		user = "You need to be in a guild, silly!"
	))?;

	let member = msg.member(ctx).await?;

	let top_role = member.get_unique_role(ctx)?;

	let color = words.string();

	match color {
		None => {
			msg.reply_report(
				ctx,
				&format!("Your current color is #{:06X}", top_role.colour.0),
			)
			.await;
		}
		Some(s) => {
			let color = if s.to_lowercase() == "random" {
				// Random color
				rand::thread_rng().gen_range(0x000000..0xFFFFFF)
			} else {
				let numbers = s.trim_start_matches('#');
				let hash = u32::from_str_radix(numbers, 16).map_err(govanerror::debug_map!(
					log fmt = ("{} does not fit in u32", numbers),
					user = "I don't know how to parse that as a color hex"
				))?;

				if hash > 0xFFFFFF {
					return Err(govanerror::debug!(
						log fmt = ("{:X} is too large", hash),
						user = "That color hex is too large! It must be between 000000 and FFFFFF"
					));
				}

				hash
			};

			let r = top_role.edit(&ctx, |e| e.colour(color as u64)).await?;

			msg.reply_report(ctx, &format!("Done. Your new color is #{:06X}", r.colour.0))
				.await;
		}
	}

	Ok(())
}
