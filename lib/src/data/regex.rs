use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
	pub static ref EMOJI_REGEX: Regex = Regex::new(r"\p{RI}\p{RI}|[\p{Emoji}--\p{Ascii}](?:\p{EMod}|\x{FE0F}\x{20E3}?|[\x{E0020}-\x{E007E}]+\x{E007F})?(?:\x{200D}\p{Emoji}(?:\p{EMod}|\x{FE0F}\x{20E3}?|[\x{E0020}-\x{E007E}]+\x{E007F})?)*|[0-9]\x{FE0F}\x{20e3}",).unwrap();
	pub static ref DISCORD_EMOJI_REGEX: Regex = Regex::new(r"<(a?):(.*?):([0-9]+)>").unwrap();
}
