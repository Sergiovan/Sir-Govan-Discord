use num_rational::Ratio;
use rand::{
	distributions::uniform::{SampleRange, SampleUniform},
	seq::SliceRandom,
	Rng,
};

pub fn from_range<T, R>(range: R) -> T
where
	T: SampleUniform,
	R: SampleRange<T>,
{
	rand::thread_rng().gen_range(range)
}

pub fn one_in(so_many: u64) -> bool {
	from_range(0..so_many) == 0
}

pub fn pick<T, L: AsRef<[T]>>(elems: &L) -> Option<&<[T] as rand::prelude::SliceRandom>::Item> {
	elems.as_ref().choose(&mut rand::thread_rng())
}

pub fn pick_or<'a, T>(elems: &'a Vec<T>, default: &'a T) -> &'a T {
	elems.choose(&mut rand::thread_rng()).unwrap_or(default)
}

type ChanceType = u32;
const CHANGE_GRANULARITY: u32 = 1_000_000;

pub struct GrabBagTier<T> {
	elems: Vec<T>,
	rarity: Ratio<ChanceType>,
}

impl<T> GrabBagTier<T> {
	pub fn maybe_new(elems: Vec<T>, rarity: Ratio<ChanceType>) -> Option<GrabBagTier<T>> {
		if elems.is_empty() || rarity == Ratio::default() {
			None
		} else {
			Some(GrabBagTier { elems, rarity })
		}
	}
}

pub struct GrabBag<T> {
	default: T,
	tiers: Vec<GrabBagTier<T>>,
}

impl<T> GrabBag<T> {
	fn total_chance(&self) -> Ratio<ChanceType> {
		self.tiers
			.last()
			.map(|x| x.rarity)
			.unwrap_or(Ratio::default())
	}

	fn picking_ratio() -> Ratio<ChanceType> {
		Ratio::new(from_range(0..CHANGE_GRANULARITY - 1), CHANGE_GRANULARITY)
	}

	fn inner_pick(&self, bias: Ratio<ChanceType>) -> Option<&T> {
		if bias == Ratio::default() {
			return None;
		}
		let choice = Self::picking_ratio() * bias;

		for tier in self.tiers.iter() {
			if choice < tier.rarity {
				return pick(&tier.elems);
			}
		}

		None
	}

	pub fn pick(&self) -> &T {
		self.pick_or(&self.default)
	}

	pub fn pick_or<'a>(&'a self, default: &'a T) -> &'a T {
		self.inner_pick(Ratio::new(1, 1)).unwrap_or(default)
	}

	pub fn pick_biased<R: Into<Ratio<ChanceType>>>(&self, bias: R) -> &T {
		self.pick_biased_or(bias, &self.default)
	}

	pub fn pick_biased_or<'a, R: Into<Ratio<ChanceType>>>(
		&'a self,
		bias: R,
		default: &'a T,
	) -> &'a T {
		self.inner_pick(bias.into()).unwrap_or(default)
	}
}

impl<'a> From<&'a GrabBag<String>> for &'a str {
	fn from(value: &'a GrabBag<String>) -> Self {
		value.pick()
	}
}

#[derive(Debug, Clone)]
pub struct GrabBagBuilderError;

pub struct GrabBagBuilder<
	T,
	const COMMON: bool,
	const UNCOMMON: bool,
	const RARE: bool,
	const MYTHICAL: bool,
	const WTF: bool,
> {
	total_chance: ChanceType,
	common: Option<GrabBagTier<T>>,
	uncommon: Option<GrabBagTier<T>>,
	rare: Option<GrabBagTier<T>>,
	mythical: Option<GrabBagTier<T>>,
	wtf: Option<GrabBagTier<T>>,
}

impl<T, const A: bool, const B: bool, const C: bool, const D: bool, const E: bool>
	GrabBagBuilder<T, A, B, C, D, E>
{
	pub fn new(chances: ChanceType) -> GrabBagBuilder<T, false, false, false, false, false> {
		if chances == 0 {
			panic!("Total chance cannot be 0");
		}

		GrabBagBuilder {
			total_chance: chances,
			common: None,
			uncommon: None,
			rare: None,
			mythical: None,
			wtf: None,
		}
	}

	fn blink<const AA: bool, const BB: bool, const CC: bool, const DD: bool, const EE: bool>(
		self,
	) -> GrabBagBuilder<T, AA, BB, CC, DD, EE> {
		GrabBagBuilder {
			total_chance: self.total_chance,
			common: self.common,
			uncommon: self.uncommon,
			rare: self.rare,
			mythical: self.mythical,
			wtf: self.wtf,
		}
	}

	pub fn finish(self, default: T) -> Result<GrabBag<T>, GrabBagBuilderError> {
		let mut collected: Ratio<ChanceType> = 0.into();
		let res = GrabBag {
			default,
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
				collected += x.rarity;

				GrabBagTier {
					elems: x.elems,
					rarity: collected,
				}
			})
			.collect(),
		};

		if res.total_chance() > Ratio::new(1, 1) {
			Err(GrabBagBuilderError)
		} else {
			Ok(res)
		}
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
