use crate::util::random::{
	self, ChanceType, GrabBag, GrabBagBuilder, GrabBagBuilderError, GrabBagTier,
};
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
}
