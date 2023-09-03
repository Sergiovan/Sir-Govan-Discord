mod grab_bag;

pub use grab_bag::*;

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
