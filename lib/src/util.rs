pub mod logger;
pub mod random;
pub mod traits;

pub fn filename_from_unicode_emoji(emoji: &str) -> String {
	let first = emoji.as_bytes().first();
	if first.is_some_and(|c| c.is_ascii_digit()) {
		format!("{:x}-20e3.png", first.unwrap())
	} else {
		format!(
			"{}.png",
			emoji
				.chars()
				.map(|c| format!("{:x}", c as u32))
				.collect::<Vec<_>>()
				.join("-")
		)
	}
}

pub fn url_from_unicode_emoji(emoji: &str) -> String {
	format!(
		"https://twemoji.maxcdn.com/v/latest/72x72/{}",
		filename_from_unicode_emoji(emoji)
	)
}

pub fn filename_from_discord_emoji(id: u64, animated: bool) -> String {
	format!("{}.{}", id, if animated { "gif" } else { "png" })
}

pub fn url_from_discord_emoji(id: u64, animated: bool) -> String {
	format!(
		"https://cdn.discordapp.com/emojis/{}",
		filename_from_discord_emoji(id, animated)
	)
}
