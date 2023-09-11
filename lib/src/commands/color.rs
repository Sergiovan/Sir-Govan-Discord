use crate::prelude::*;

use crate::bot::Bot;
use serenity::builder::EditRole;
use serenity::model::prelude::*;
use serenity::prelude::*;

use super::commander::Arguments;

use sirgovan_macros::command;

use rand::Rng;

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

	let mut top_role = member.get_unique_role(ctx)?;

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

			top_role
				.edit(&ctx, EditRole::default().colour(color as u64))
				.await?;

			msg.reply_report(ctx, &format!("Done. Your new color is #{:06X}", color))
				.await;
		}
	}

	Ok(())
}
