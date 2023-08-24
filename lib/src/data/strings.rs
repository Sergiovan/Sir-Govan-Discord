use crate::util::random::{ChanceType, GrabBag, GrabBagBuilder, GrabBagBuilderError, GrabBagTier};
use num_rational::Ratio;

use serde::Deserialize;

pub type StringBag = GrabBag<String>;

#[derive(Deserialize)]
pub struct SingleString {
	default: Option<String>,
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
	pub nickname: SingleString,

	pub tweet_retweeter: SingleString,
	pub tweet_fact_check: SingleString,
	pub tweet_month: SingleString,
	pub tweet_client: SingleString,
	pub tweet_esoteric_amount_prefix: SingleString,
	pub tweet_esoteric_amount_suffix: SingleString,
	pub tweet_amount_symbol: SingleString,
	pub tweet_esoteric_time: SingleString,
	pub tweet_username: SingleString,
	pub tweet_extra_reply: SingleString,
	pub tweet_extra_text: SingleString,
}
