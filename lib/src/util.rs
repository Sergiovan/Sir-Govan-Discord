pub mod error;
pub mod logger;
pub mod random;
pub mod traits;

use regex::{Captures, Regex};

pub fn filename_from_unicode_emoji(emoji: &str) -> String {
	let first = emoji.as_bytes().first();
	if first.is_some_and(|c| c.is_ascii_digit()) {
		format!("{:x}-20e3.png", first.unwrap())
	} else {
		format!(
			"{}.png",
			emoji
				.trim_end_matches('\u{FE0F}')
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

pub fn replace_all(
	re: &Regex,
	haystack: &str,
	replacement: impl Fn(&Captures) -> String,
) -> String {
	let mut new = String::with_capacity(haystack.len());
	let mut last_match = 0;
	for caps in re.captures_iter(haystack) {
		let m = caps.get(0).unwrap();
		new.push_str(&haystack[last_match..m.start()]);
		new.push_str(&replacement(&caps));
		last_match = m.end();
	}
	new.push_str(&haystack[last_match..]);
	new
}
