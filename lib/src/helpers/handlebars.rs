pub struct Handlebar<'a> {
	handlebar: handlebars::Handlebars<'a>,
}

impl<'a> Handlebar<'a> {
	const ALWAYS_SUNNY: &str = "always_sunny";
	const FAKE_TWITTER: &str = "fake_twitter";

	pub fn new() -> anyhow::Result<Handlebar<'a>> {
		let mut handlebar = handlebars::Handlebars::new();

		handlebar.register_template_file(Self::ALWAYS_SUNNY, "res/html/titlecard.hbs")?;
		handlebar.register_template_file(Self::FAKE_TWITTER, "res/html/tweet.hbs")?;

		Ok(Handlebar { handlebar })
	}

	pub fn always_sunny(&self, data: ()) -> Result<String, handlebars::RenderError> {
		self.handlebar.render(Self::ALWAYS_SUNNY, &data)
	}

	pub fn twitter(&self, data: TweetData) -> Result<String, handlebars::RenderError> {
		self.handlebar.render(Self::FAKE_TWITTER, &data)
	}
}

#[derive(serde::Serialize)]
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
