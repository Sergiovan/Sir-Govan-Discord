use lazy_static::lazy_static;

use crate::util::random::{GrabBag, GrabBagBuilder, GrabBagTier};

pub struct Handlebar<'a> {
	handlebar: handlebars::Handlebars<'a>,
}

impl<'a> Handlebar<'a> {
	const ALWAYS_SUNNY: &str = "always_sunny";
	const FAKE_TWITTER: &str = "fake_twitter";

	pub fn new() -> anyhow::Result<Handlebar<'a>> {
		use crate::data::config;
		use std::path::Path;
		let mut handlebar = handlebars::Handlebars::new();

		handlebar.register_template_file(
			Self::ALWAYS_SUNNY,
			Path::new(config::RESOURCE_PATH)
				.join(config::HTML_DIR)
				.join(config::ALWAYS_SUNNY_HBS),
		)?;

		handlebar.register_template_file(
			Self::FAKE_TWITTER,
			Path::new(config::RESOURCE_PATH)
				.join(config::HTML_DIR)
				.join(config::FAKE_TWITTER_HBS),
		)?;

		Ok(Handlebar { handlebar })
	}

	pub fn always_sunny(&self, data: ()) -> Result<String, handlebars::RenderError> {
		self.handlebar.render(Self::ALWAYS_SUNNY, &data)
	}

	pub fn twitter(&self, data: TweetData) -> Result<String, handlebars::RenderError> {
		self.handlebar.render(Self::FAKE_TWITTER, &data)
	}
}

lazy_static! {
	pub static ref TWEET_THEME_GRAB_BAG: GrabBag<TweetTheme> = GrabBagBuilder::new()
		.rare(GrabBagTier::maybe_rare(Some(vec![
			TweetTheme::Light,
			TweetTheme::Dark
		])))
		.finish(Some(TweetTheme::Dim), None)
		.unwrap();
}

#[derive(serde::Serialize, Clone)]
pub enum TweetTheme {
	#[serde(rename = "dim")]
	Dim,
	#[serde(rename = "light")]
	Light,
	#[serde(rename = "dark")]
	Dark,
}

#[derive(serde::Serialize)]
pub struct TweetData {
	pub retweeter: String,
	pub avatar: String,
	pub name: String,
	pub verified: bool,
	pub at: String,
	pub tweet_text: String,
	pub hour: String,
	pub month: String,
	pub day: String,
	pub year: String,
	pub client: String,
	pub any_numbers: bool,
	pub retweets: Option<String>,
	pub quotes: Option<String>,
	pub likes: Option<String>,
	pub more_tweets: Vec<TweetMoreData>,

	pub theme: Option<TweetTheme>,
	pub reply_to: Option<String>,
	pub image: Option<String>,
	pub fact_check: Option<String>,
}

#[derive(serde::Serialize)]
pub struct TweetMoreData {
	pub avatar: String,
	pub name: String,
	pub verified: bool,
	pub at: String,
	pub time: String,
	pub tweet_text: String,
	pub replies: String,
	pub retweets: String,
	pub likes: String,

	pub reply_to: Option<String>,
	pub image: Option<String>,
}
