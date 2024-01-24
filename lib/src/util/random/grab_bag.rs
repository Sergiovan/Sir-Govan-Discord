use num_rational::Ratio;
use thiserror::Error;

pub type ChanceType = u32;
const CHANGE_GRANULARITY: u32 = 1_000_000;

pub const DEFAULT_CHANCE_TOTAL: ChanceType = 10000; // 100%
pub const COMMON_CHANCE: Ratio<ChanceType> = Ratio::new_raw(7500, DEFAULT_CHANCE_TOTAL); // 75%
pub const UNCOMMON_CHANCE: Ratio<ChanceType> = Ratio::new_raw(2000, DEFAULT_CHANCE_TOTAL); // 20%
pub const RARE_CHANCE: Ratio<ChanceType> = Ratio::new_raw(400, DEFAULT_CHANCE_TOTAL); // 4%
pub const MYTHICAL_CHANCE: Ratio<ChanceType> = Ratio::new_raw(99, DEFAULT_CHANCE_TOTAL); // 0.99%
pub const WTF_CHANCE: Ratio<ChanceType> = Ratio::new_raw(1, DEFAULT_CHANCE_TOTAL); // 0.01%

pub trait RandomBag {
	type Item<'a>
	where
		Self: 'a;

	fn pick(&self) -> Self::Item<'_> {
		self.pick_biased(Ratio::new(1, 1))
	}

	fn pick_biased(&self, bias: Ratio<ChanceType>) -> Self::Item<'_>;
}

pub struct GrabBagTier<T> {
	elems: Vec<T>,
	rarity: Ratio<ChanceType>,
}

impl<T> GrabBagTier<T> {
	pub fn maybe_new(elems: Option<Vec<T>>, rarity: Ratio<ChanceType>) -> Option<GrabBagTier<T>> {
		let vec = elems?;
		if vec.is_empty() || rarity == Ratio::default() {
			None
		} else {
			Some(GrabBagTier { elems: vec, rarity })
		}
	}

	pub fn maybe_common(elems: Option<Vec<T>>) -> Option<GrabBagTier<T>> {
		Self::maybe_new(elems, COMMON_CHANCE.reduced())
	}

	pub fn maybe_uncommon(elems: Option<Vec<T>>) -> Option<GrabBagTier<T>> {
		Self::maybe_new(elems, UNCOMMON_CHANCE.reduced())
	}

	pub fn maybe_rare(elems: Option<Vec<T>>) -> Option<GrabBagTier<T>> {
		Self::maybe_new(elems, RARE_CHANCE.reduced())
	}

	pub fn maybe_mythical(elems: Option<Vec<T>>) -> Option<GrabBagTier<T>> {
		Self::maybe_new(elems, MYTHICAL_CHANCE.reduced())
	}

	pub fn maybe_wtf(elems: Option<Vec<T>>) -> Option<GrabBagTier<T>> {
		Self::maybe_new(elems, WTF_CHANCE.reduced())
	}
}

static_assertions::const_assert_eq!(
	DEFAULT_CHANCE_TOTAL,
	*COMMON_CHANCE.numer()
		+ *UNCOMMON_CHANCE.numer()
		+ *RARE_CHANCE.numer()
		+ *MYTHICAL_CHANCE.numer()
		+ *WTF_CHANCE.numer()
);

#[derive(Default)]
struct GrabBagInner<T> {
	tiers: Vec<GrabBagTier<T>>,
}

impl<T> GrabBagInner<T> {
	fn total_chance(&self) -> Ratio<ChanceType> {
		self.tiers.last().map(|x| x.rarity).unwrap_or_default()
	}

	fn picking_ratio() -> Ratio<ChanceType> {
		Ratio::new(
			super::from_range(0..CHANGE_GRANULARITY - 1),
			CHANGE_GRANULARITY,
		)
	}
}

impl<T> RandomBag for GrabBagInner<T> {
	type Item<'a> = Option<&'a T> where T: 'a;

	fn pick_biased(&self, bias: Ratio<ChanceType>) -> Self::Item<'_> {
		if bias == Ratio::default() {
			return None;
		}
		let choice = Self::picking_ratio() * bias.recip();

		for tier in self.tiers.iter() {
			if choice < tier.rarity {
				return super::pick(&tier.elems);
			}
		}

		None
	}
}

#[derive(Default)]
pub struct GrabBagLoose<T> {
	inner: GrabBagInner<T>,
}

impl<T> GrabBagLoose<T> {
	pub fn pick_or<'a>(&'a self, default: &'a T) -> &'a T {
		self.inner.pick().unwrap_or(default)
	}

	pub fn pick_biased_or<'a, R>(&'a self, bias: R, default: &'a T) -> &'a T
	where
		R: Into<Ratio<ChanceType>>,
	{
		self.inner.pick_biased(bias.into()).unwrap_or(default)
	}
}

impl<T> RandomBag for GrabBagLoose<T> {
	type Item<'a> = Option<&'a T> where T: 'a;

	fn pick_biased(&self, bias: Ratio<ChanceType>) -> Self::Item<'_> {
		self.inner.pick_biased(bias)
	}
}

impl<T> From<GrabBagInner<T>> for GrabBagLoose<T> {
	fn from(value: GrabBagInner<T>) -> Self {
		GrabBagLoose { inner: value }
	}
}

#[derive(Default)]
pub struct GrabBag<T> {
	inner: GrabBagInner<T>,
	default: T,
}

impl<T> RandomBag for GrabBag<T> {
	type Item<'a> = &'a T where T: 'a;

	fn pick_biased(&self, bias: Ratio<ChanceType>) -> Self::Item<'_> {
		self.inner.pick_biased(bias).unwrap_or(&self.default)
	}
}

#[derive(Debug, Clone, Error)]
pub struct GrabBagBuilderError(Ratio<ChanceType>);

impl std::fmt::Display for GrabBagBuilderError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Ratio was greater than 1: {}", self.0)
	}
}

pub struct GrabBagBuilder<
	T,
	const COMMON: bool,
	const UNCOMMON: bool,
	const RARE: bool,
	const MYTHICAL: bool,
	const WTF: bool,
> {
	common: Option<GrabBagTier<T>>,
	uncommon: Option<GrabBagTier<T>>,
	rare: Option<GrabBagTier<T>>,
	mythical: Option<GrabBagTier<T>>,
	wtf: Option<GrabBagTier<T>>,
}

impl<T> GrabBagBuilder<T, false, false, false, false, false> {
	pub fn new() -> GrabBagBuilder<T, false, false, false, false, false> {
		GrabBagBuilder {
			common: None,
			uncommon: None,
			rare: None,
			mythical: None,
			wtf: None,
		}
	}
}

impl<T> Default for GrabBagBuilder<T, false, false, false, false, false> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T, const A: bool, const B: bool, const C: bool, const D: bool, const E: bool>
	GrabBagBuilder<T, A, B, C, D, E>
{
	fn blink<const AA: bool, const BB: bool, const CC: bool, const DD: bool, const EE: bool>(
		self,
	) -> GrabBagBuilder<T, AA, BB, CC, DD, EE> {
		GrabBagBuilder {
			common: self.common,
			uncommon: self.uncommon,
			rare: self.rare,
			mythical: self.mythical,
			wtf: self.wtf,
		}
	}

	fn finish_inner(
		self,
		chance_modifier: Option<Ratio<ChanceType>>,
	) -> Result<GrabBagInner<T>, GrabBagBuilderError> {
		let mut collected: Ratio<ChanceType> = 0.into();
		let inner = GrabBagInner {
			tiers: [
				self.common,
				self.uncommon,
				self.rare,
				self.mythical,
				self.wtf,
			]
			.into_iter()
			.flatten()
			.map(move |x| {
				collected += x.rarity * chance_modifier.unwrap_or(Ratio::new(1, 1));

				GrabBagTier {
					elems: x.elems,
					rarity: collected,
				}
			})
			.collect(),
		};

		if inner.total_chance() > Ratio::new(1, 1) {
			Err(GrabBagBuilderError(inner.total_chance()))
		} else {
			Ok(inner)
		}
	}

	pub fn finish_loose(
		self,
		chance_modifier: Option<Ratio<ChanceType>>,
	) -> Result<GrabBagLoose<T>, GrabBagBuilderError> {
		Ok(self.finish_inner(chance_modifier)?.into())
	}

	pub fn finish(
		self,
		default: T,
		chance_modifier: Option<Ratio<ChanceType>>,
	) -> Result<GrabBag<T>, GrabBagBuilderError> {
		Ok(GrabBag {
			inner: self.finish_inner(chance_modifier)?,
			default,
		})
	}
}

impl<T, const B: bool, const C: bool, const D: bool, const E: bool>
	GrabBagBuilder<T, false, B, C, D, E>
{
	pub fn common(mut self, tier: Option<GrabBagTier<T>>) -> GrabBagBuilder<T, true, B, C, D, E> {
		self.common = tier;

		self.blink::<true, B, C, D, E>()
	}
}

impl<T, const A: bool, const C: bool, const D: bool, const E: bool>
	GrabBagBuilder<T, A, false, C, D, E>
{
	pub fn uncommon(mut self, tier: Option<GrabBagTier<T>>) -> GrabBagBuilder<T, A, true, C, D, E> {
		self.uncommon = tier;

		self.blink::<A, true, C, D, E>()
	}
}

impl<T, const A: bool, const B: bool, const D: bool, const E: bool>
	GrabBagBuilder<T, A, B, false, D, E>
{
	pub fn rare(mut self, tier: Option<GrabBagTier<T>>) -> GrabBagBuilder<T, A, B, true, D, E> {
		self.rare = tier;

		self.blink::<A, B, true, D, E>()
	}
}

impl<T, const A: bool, const B: bool, const C: bool, const E: bool>
	GrabBagBuilder<T, A, B, C, false, E>
{
	pub fn mythical(mut self, tier: Option<GrabBagTier<T>>) -> GrabBagBuilder<T, A, B, C, true, E> {
		self.mythical = tier;

		self.blink::<A, B, C, true, E>()
	}
}

impl<T, const A: bool, const B: bool, const C: bool, const D: bool>
	GrabBagBuilder<T, A, B, C, D, false>
{
	pub fn wtf(mut self, tier: Option<GrabBagTier<T>>) -> GrabBagBuilder<T, A, B, C, D, true> {
		self.wtf = tier;

		self.blink::<A, B, C, D, true>()
	}
}
