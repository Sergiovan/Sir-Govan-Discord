use std::error::Error;

use anyhow::bail;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::Browser;
use std::sync::Arc;

use lazy_static::lazy_static;

pub struct Screenshotter {
	browser: Browser,
	handlebars: super::handlebars::Handlebar<'static>,
}

impl Screenshotter {
	pub fn new() -> Result<Screenshotter, anyhow::Error> {
		use std::ffi::OsStr;

		lazy_static! {
			static ref ARGS: Vec<&'static OsStr> = vec![
				OsStr::new("--autoplay-policy=user-gesture-required"),
				OsStr::new("--disable-background-networking"),
				OsStr::new("--disable-background-timer-throttling"),
				OsStr::new("--disable-backgrounding-occluded-windows"),
				OsStr::new("--disable-breakpad"),
				OsStr::new("--disable-client-side-phishing-detection"),
				OsStr::new("--disable-component-update"),
				OsStr::new("--disable-default-apps"),
				OsStr::new("--disable-dev-shm-usage"),
				OsStr::new("--disable-domain-reliability"),
				OsStr::new("--disable-extensions"),
				OsStr::new("--disable-features=AudioServiceOutOfProcess"),
				OsStr::new("--disable-hang-monitor"),
				OsStr::new("--disable-ipc-flooding-protection"),
				OsStr::new("--disable-notifications"),
				OsStr::new("--disable-offer-store-unmasked-wallet-cards"),
				OsStr::new("--disable-popup-blocking"),
				OsStr::new("--disable-print-preview"),
				OsStr::new("--disable-prompt-on-repost"),
				OsStr::new("--disable-renderer-backgrounding"),
				OsStr::new("--disable-setuid-sandbox"),
				OsStr::new("--disable-speech-api"),
				OsStr::new("--disable-sync"),
				OsStr::new("--hide-scrollbars"),
				OsStr::new("--ignore-gpu-blacklist"),
				OsStr::new("--metrics-recording-only"),
				OsStr::new("--mute-audio"),
				OsStr::new("--no-default-browser-check"),
				OsStr::new("--no-first-run"),
				OsStr::new("--no-pings"),
				OsStr::new("--no-sandbox"),
				OsStr::new("--no-zygote"),
				OsStr::new("--disable-gpu"),
				OsStr::new("--password-store=basic"),
				OsStr::new("--use-gl=swiftshader"),
				OsStr::new("--use-mock-keychain"),
			];
		}

		let handlebars = super::handlebars::Handlebar::new()?;

		let executable = match headless_chrome::browser::default_executable() {
			Ok(exe) => exe,
			Err(e) => bail!("{}", e),
		};

		let launch_options = headless_chrome::LaunchOptions::default_builder()
			.path(Some(executable))
			.sandbox(false)
			.idle_browser_timeout(std::time::Duration::from_secs(u64::MAX))
			.args(ARGS.clone())
			.build()?;

		let browser = Browser::new(launch_options)?;

		Ok(Screenshotter {
			browser,
			handlebars,
		})
	}
}

impl Screenshotter {
	pub fn take_screenshot(
		&self,
		page: &str,
		width: Option<f64>,
		height: Option<f64>,
	) -> Result<Vec<u8>, Box<dyn Error>> {
		let tab = self.browser.new_tab()?;

		tab.set_bounds(headless_chrome::types::Bounds::Normal {
			left: None,
			top: None,
			width: width.or(Some(1920_f64)),
			height: height.or(Some(1920_f64)),
		})?;

		tab.navigate_to(page)?.wait_until_navigated()?;

		let bytes = tab
			.wait_for_element("body")
			.expect("No body to capture")
			.capture_screenshot(Page::CaptureScreenshotFormatOption::Png)?;

		Ok(bytes)
	}

	pub fn screenshot_from_html(
		&self,
		html: &str,
		capture: &str,
		width: Option<f64>,
		height: Option<f64>,
	) -> anyhow::Result<Vec<u8>> {
		let tab = self.browser.new_tab()?;
		tab.evaluate(
			&format!(
				r#"(function(){{
      let html = `{}`;

      document.open();
      document.write(html);
      document.close();
    }})()"#,
				html
			),
			false,
		)
		.expect("Could not load js");

		let capture = tab.wait_for_element(capture)?;
		let capture_box = capture.get_box_model()?;

		let min_width = width.unwrap_or(0_f64);
		const MAX_WIDTH: f64 = 1920_f64;
		let width = Some(capture_box.width.clamp(min_width, MAX_WIDTH));

		let min_height = height.unwrap_or(0_f64);
		const MAX_HEIGHT: f64 = 1080_f64;
		let height = Some(capture_box.height.clamp(min_height, MAX_HEIGHT));

		tab.set_bounds(headless_chrome::types::Bounds::Normal {
			left: None,
			top: None,
			width,
			height,
		})?;

		let bytes = capture.capture_screenshot(Page::CaptureScreenshotFormatOption::Png)?;

		Ok(bytes)
	}

	pub async fn twitter(
		&self,
		tweet_data: super::handlebars::TweetData,
	) -> anyhow::Result<Vec<u8>> {
		let html = self.handlebars.twitter(tweet_data)?;

		tokio::join!(async move {
			self.screenshot_from_html(&html, ".fake-twitter", Some(510.0), Some(10.0))
		})
		.0
	}

	pub async fn always_sunny(
		&self,
		always_sunny_data: super::handlebars::AlwaysSunnyData,
	) -> anyhow::Result<Vec<u8>> {
		let html = self.handlebars.always_sunny(always_sunny_data)?;

		tokio::join!(async move { self.screenshot_from_html(&html, ".container", None, None) }).0
	}
}
