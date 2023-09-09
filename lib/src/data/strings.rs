use crate::util::random::{
	ChanceType, GrabBag, GrabBagBuilder, GrabBagBuilderError, GrabBagLoose, GrabBagTier,
};
use num_rational::Ratio;

use serde::Deserialize;

pub type StringBag = GrabBag<String>;
pub type StringBagLoose = GrabBagLoose<String>;

#[derive(Deserialize)]
pub struct SingleStringLoose {
	modifier: Option<Ratio<ChanceType>>,
	common: Option<Vec<String>>,
	uncommon: Option<Vec<String>>,
	rare: Option<Vec<String>>,
	mythical: Option<Vec<String>>,
	wtf: Option<Vec<String>>,
}

impl TryFrom<SingleStringLoose> for StringBagLoose {
	type Error = GrabBagBuilderError;

	fn try_from(value: SingleStringLoose) -> Result<Self, Self::Error> {
		GrabBagBuilder::new()
			.common(GrabBagTier::maybe_common(value.common))
			.uncommon(GrabBagTier::maybe_uncommon(value.uncommon))
			.rare(GrabBagTier::maybe_rare(value.rare))
			.mythical(GrabBagTier::maybe_mythical(value.mythical))
			.wtf(GrabBagTier::maybe_wtf(value.wtf))
			.finish_loose(value.modifier)
	}
}

#[derive(Deserialize)]
pub struct SingleString {
	default: String,
	modifier: Option<Ratio<ChanceType>>,
	common: Option<Vec<String>>,
	uncommon: Option<Vec<String>>,
	rare: Option<Vec<String>>,
	mythical: Option<Vec<String>>,
	wtf: Option<Vec<String>>,
}

impl TryFrom<SingleString> for StringBag {
	type Error = GrabBagBuilderError;

	fn try_from(value: SingleString) -> Result<Self, Self::Error> {
		GrabBagBuilder::new()
			.common(GrabBagTier::maybe_common(value.common))
			.uncommon(GrabBagTier::maybe_uncommon(value.uncommon))
			.rare(GrabBagTier::maybe_rare(value.rare))
			.mythical(GrabBagTier::maybe_mythical(value.mythical))
			.wtf(GrabBagTier::maybe_wtf(value.wtf))
			.finish(value.default, value.modifier)
	}
}

#[derive(Deserialize)]
pub struct Strings {
	pub nickname: SingleStringLoose,

	pub generic_error: SingleString,

	pub status_playing: SingleString,
	pub status_watching: SingleString,
	pub status_listening: SingleString,

	pub ping: SingleString,

	pub roll: SingleString,

	pub tweet_retweeter: SingleStringLoose,
	pub tweet_fact_check: SingleStringLoose,
	pub tweet_month: SingleStringLoose,
	pub tweet_client: SingleString,
	pub tweet_esoteric_amount_prefix: SingleStringLoose,
	pub tweet_esoteric_amount_suffix: SingleStringLoose,
	pub tweet_amount_symbol: SingleStringLoose,
	pub tweet_esoteric_time: SingleStringLoose,
	pub tweet_username: SingleStringLoose,
	pub tweet_extra_reply: SingleStringLoose,
	pub tweet_extra_text: SingleStringLoose,

	pub titlecard_song: SingleString,
	pub titlecard_show_prefix: SingleString,
	pub titlecard_show_entire: SingleString,
}
