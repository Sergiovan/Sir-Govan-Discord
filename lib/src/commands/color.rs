use std::num::ParseIntError;

use crate::{prelude::*, util::traits::UniqueRoleError};

use crate::bot::Bot;
use serenity::model::prelude::*;
use serenity::prelude::*;

use super::commander::{Arguments, CommandResult};

use sirgovan_macros::command;

use rand::Rng;

#[derive(thiserror::Error, Debug)]
enum ColorError {
	#[error("")]
	NotInGuild,
	#[error("Could not get member from {0}: {1}")]
	MemberFailure(UserId, #[source] anyhow::Error),
	#[error("{0}")]
	NoUniqueRole(#[from] UniqueRoleError),
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
			Self::NoUniqueRole(e) => e.to_user(),
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
) -> CommandResult<ColorError> {
	msg.guild_id.ok_or(ColorError::NotInGuild)?;

	let member = msg
		.member(ctx)
		.await
		.map_err(|e| ColorError::MemberFailure(msg.author.id, e.into()))?;

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
				let hash = u32::from_str_radix(numbers, 16)?;

				if hash > 0xFFFFFF {
					return Err(ColorError::HexTooLarge);
				}

				hash
			};

			let r = top_role
				.edit(&ctx, |e| e.colour(color as u64))
				.await
				.map_err(|e| {
					ColorError::RoleEditError(e, top_role.id, member.user.id, color as u64)
				})?;

			msg.reply_report(ctx, &format!("Done. Your new color is #{:06X}", r.colour.0))
				.await;
		}
	}

	Ok(())
}
