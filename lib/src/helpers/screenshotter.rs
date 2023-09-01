use fantoccini::wd::Capabilities;

const ARGS: &[&str] = &[
	"--autoplay-policy=user-gesture-required",
	"--disable-background-networking",
	"--disable-background-timer-throttling",
	"--disable-backgrounding-occluded-windows",
	"--disable-breakpad",
	"--disable-client-side-phishing-detection",
	"--disable-component-update",
	"--disable-default-apps",
	"--disable-dev-shm-usage",
	"--disable-domain-reliability",
	"--disable-extensions",
	"--disable-features=AudioServiceOutOfProcess",
	"--disable-hang-monitor",
	"--disable-ipc-flooding-protection",
	"--disable-notifications",
	"--disable-offer-store-unmasked-wallet-cards",
	"--disable-popup-blocking",
	"--disable-print-preview",
	"--disable-prompt-on-repost",
	"--disable-renderer-backgrounding",
	"--disable-setuid-sandbox",
	"--disable-speech-api",
	"--disable-sync",
	"--hide-scrollbars",
	"--ignore-gpu-blacklist",
	"--metrics-recording-only",
	"--mute-audio",
	"--no-default-browser-check",
	"--no-first-run",
	"--no-pings",
	"--headless=new",
	"--no-sandbox",
	// "--no-zygote",
	"--disable-gpu",
	"--password-store=basic",
	"--use-gl=swiftshader",
	"--use-mock-keychain",
];

pub struct Screenshotter {
	client: fantoccini::Client,
	handlebars: super::handlebars::Handlebar<'static>,
}

impl Screenshotter {
	pub async fn new() -> anyhow::Result<Screenshotter> {
		let handlebars = super::handlebars::Handlebar::new()?;

		Ok(Screenshotter {
			client: Self::new_connection().await?,
			handlebars,
		})
	}

	async fn new_connection() -> anyhow::Result<fantoccini::Client> {
		let capability_array = ARGS
			.iter()
			.map(|s| format!(r#""{}""#, s))
			.collect::<Vec<_>>()
			.join(", ");

		let cap: Capabilities = serde_json::from_str(&format!(
			r#"{{"browserName":"chrome", "goog:chromeOptions":{{"args":[{}]}}}}"#,
			capability_array
		))
		.unwrap();

		Ok(fantoccini::ClientBuilder::native()
			.capabilities(cap)
			.connect("http://localhost:9515")
			.await?)
	}

	pub async fn reconnect(&mut self) -> anyhow::Result<()> {
		self.client = Self::new_connection().await?;

		Ok(())
	}

	pub async fn screenshot_from_html(
		&self,
		html: &str,
		capture: &str,
		width: Option<f64>,
		height: Option<f64>,
	) -> anyhow::Result<Vec<u8>> {
		let html = openssl::base64::encode_block(html.as_bytes());

		self.client
			.goto(&format!("data:text/html;base64,{}", html))
			.await?;

		let elem = self
			.client
			.wait()
			.for_element(fantoccini::Locator::Css(capture))
			.await?;

		let (.., w, h) = elem.rectangle().await?;

		const HEADER_SIZE: f64 = 123_f64;
		let min_width = width.unwrap_or(0_f64);
		const MAX_WIDTH: f64 = 1920_f64;

		let min_height = height.unwrap_or(0_f64);
		const MAX_HEIGHT: f64 = 1080_f64 + HEADER_SIZE; // To account for top bar

		self.client
			.set_window_rect(
				0,
				0,
				w.clamp(min_width, MAX_WIDTH) as u32,
				(h + HEADER_SIZE).clamp(min_height, MAX_HEIGHT) as u32,
			)
			.await?;

		let bytes = elem.screenshot().await?;

		Ok(bytes)
	}

	pub async fn twitter(
		&self,
		tweet_data: super::handlebars::TweetData,
	) -> anyhow::Result<Vec<u8>> {
		let html = self.handlebars.twitter(tweet_data)?;

		self.screenshot_from_html(&html, ".fake-twitter", None, None)
			.await
	}

	pub async fn always_sunny(
		&self,
		always_sunny_data: super::handlebars::AlwaysSunnyData,
	) -> anyhow::Result<Vec<u8>> {
		let html = self.handlebars.always_sunny(always_sunny_data)?;

		self.screenshot_from_html(&html, ".container", None, None)
			.await
	}
}
