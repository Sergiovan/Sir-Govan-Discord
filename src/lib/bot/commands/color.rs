use std::convert::Infallible;

use crate::bot::commands::commander::Command;
use crate::util::{self, logger, NickOrName, ResultErrorHandler, UniqueColorError};
use serenity::model::prelude::*;
use serenity::{async_trait, prelude::*};

use super::commander::Arguments;

use sirgovan_macros::command;

use rand::Rng;

#[command(aliases = ["colour"])]
fn color<'a>(
	&self,
	ctx: &Context,
	msg: &'a Message,
	mut words: Arguments<'a>,
) -> Option<Infallible> {
	msg.guild_id?;

	let member = msg
		.member(ctx)
		.await
		.unwrap_or_log(&format!("Could not fetch member from message {}", msg.id))?;

	let top_role = match util::get_unique_color(ctx, &member) {
		Ok(r) => r,
		Err(e) => match e {
			UniqueColorError::GuildMissing => {
				logger::error(&format!(
					"Error finding guild from member {} ({})",
					member.get_name(),
					member.user.id
				));
				return None;
			}
			UniqueColorError::RolesMissing => {
				logger::error(&format!(
					"Error getting roles from member {} ({})",
					member.get_name(),
					member.user.id
				));
				return None;
			}
			UniqueColorError::NoColoredRole => {
				msg.reply(&ctx, "It seems you have no proper role to color")
					.await
					.log_if_err(&format!("Error replying to {}", msg.id));
				return None;
			}
		},
	};

	let color = words.string();

	match color {
		None => {
			// We say
			msg.reply(
				&ctx,
				&format!("Your current color is #{:06X}", top_role.colour.0),
			)
			.await
			.unwrap_or_log(&format!("Error replying to {}", msg.id))?;
		}
		Some(s) => {
			let color = if s.to_lowercase() == "random" {
				// Random color
				rand::thread_rng().gen_range(0x000000..0xFFFFFF)
			} else {
				let numbers = s.trim_start_matches('#');
				let Ok(hash) = u32::from_str_radix(numbers, 16) else {
                        msg.reply(&ctx, "That's an invalid hex").await.log_if_err(&format!("Error replying to {}", msg.id));
                        return None;
                    };
				if hash > 0xFFFFFF {
					msg.reply(&ctx, "That hex is too large")
						.await
						.log_if_err(&format!("Error replying to {}", msg.id));
					return None;
				}
				hash
			};

			match top_role.edit(&ctx, |e| e.colour(color as u64)).await {
				Ok(r) => msg
					.reply(
						&ctx,
						&format!("Done. Your new color is #{:06X}", r.colour.0),
					)
					.await
					.unwrap_or_log(&format!("Error replying to {}", msg.id))?,
				Err(e) => {
					logger::error(&format!(
						"Could not change role {} for {} to color #{:06X}: {}",
						top_role.name,
						member.get_name(),
						color,
						e
					));
					msg.reply(
						&ctx,
						"Something went wrong. Could not change your role color",
					)
					.await
					.unwrap_or_log(&format!("Error replying to {}", msg.id))?
				}
			};
		}
	}

	None
}
