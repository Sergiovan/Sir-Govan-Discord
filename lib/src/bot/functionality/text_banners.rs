use std::{convert::Infallible, fs};

use crate::util::filename_from_discord_emoji;

use skia_safe;

use std::ops::Mul;
use std::path;

use crate::{
	bot::data,
	util::{logger, MatchMap},
};

pub struct Rgb(u8, u8, u8);

impl Mul<Self> for &Rgb {
	type Output = Rgb;
	fn mul(self, rhs: Self) -> Self::Output {
		let r = self.0 as f32 * rhs.0 as f32 / 255_f32;
		let g = self.1 as f32 * rhs.1 as f32 / 255_f32;
		let b = self.2 as f32 * rhs.2 as f32 / 255_f32;

		Rgb(
			r.clamp(0_f32, 255_f32) as u8,
			g.clamp(0_f32, 255_f32) as u8,
			b.clamp(0_f32, 255_f32) as u8,
		)
	}
}

impl From<&Rgb> for skia_safe::Color4f {
	fn from(value: &Rgb) -> Self {
		skia_safe::Color4f::new(
			value.0 as f32 / 255_f32,
			value.1 as f32 / 255_f32,
			value.2 as f32 / 255_f32,
			1_f32,
		)
	}
}

pub enum Font {
	Garamond,
	_Optimus,
}

pub enum FontWeight {
	Normal,
	_Bold,
}

pub struct Preset {
	pub main_color: Rgb,
	pub sheen_tint: Rgb,

	pub text_spacing: f32,
	pub text_opacity: Option<f32>,

	pub sheen_size: f32,
	pub sheen_opacity: f32,

	pub shadow_opacity: Option<f32>,

	pub font: Font,
	pub font_weight: Option<FontWeight>,
}

impl Preset {
	pub const _HUMANITY_RESTORED: Preset = Preset {
		main_color: Rgb(129, 187, 153),
		sheen_tint: Rgb(255, 178, 153),

		text_spacing: 8_f32,
		sheen_size: 1.1_f32,
		sheen_opacity: 0.08_f32,

		text_opacity: None,
		shadow_opacity: None,

		font: Font::Garamond,
		font_weight: None,
	};
	pub const _VICTORY_ACHIEVED: Preset = Preset {
		main_color: Rgb(255, 255, 107),
		sheen_tint: Rgb(187, 201, 192),

		text_spacing: 0_f32,
		sheen_size: 1.16,
		sheen_opacity: 0.08,

		text_opacity: None,
		shadow_opacity: None,

		font: Font::Garamond,
		font_weight: None,
	};
	pub const BONFIRE_LIT: Preset = Preset {
		main_color: Rgb(255, 228, 92),
		sheen_tint: Rgb(251, 149, 131),

		text_spacing: 1_f32,
		sheen_size: 1.14,
		sheen_opacity: 0.1,

		text_opacity: None,
		shadow_opacity: None,

		font: Font::Garamond,
		font_weight: None,
	};
	pub const _YOU_DIED: Preset = Preset {
		main_color: Rgb(101, 5, 4),
		sheen_tint: Rgb(0, 0, 0),

		text_spacing: 8_f32,
		sheen_size: 0.0,
		sheen_opacity: 0.0,

		text_opacity: Some(1.0),
		shadow_opacity: Some(1.0),

		font: Font::_Optimus,
		font_weight: Some(FontWeight::_Bold),
	};
}

enum LinePart {
	String(String),
	Image(path::PathBuf, bool),
}

type LineData = Vec<LinePart>;
type CaptionData = Vec<LineData>;

const Y_SCALE: f32 = 1.5_f32;
const FONT_SIZE: u32 = 92;

fn size_guesstimation(text: &str, type_face: &skia_safe::Typeface) -> (f32, f32) {
	let font = skia_safe::Font::new(type_face, Some(FONT_SIZE as f32));

	let longest = text
		.split('\n')
		.map(|l| {
			font.measure_str(
				crate::util::DISCORD_EMOJI_REGEX.replace(l.trim(), "XX"),
				None,
			)
			.0
		})
		.reduce(f32::max)
		.unwrap_or(1200_f32);

	((longest * 1.1_f32).clamp(1200_f32, 1920_f32), 280_f32)
}

pub async fn create_image(text: &str, preset: &Preset) -> Result<skia_safe::Data, &'static str> {
	// let ttf = fs::read(match preset.font {
	// 	Font::Garamond => "res/media/Adobe Garamond Pro Regular.ttf",
	// 	Font::Optimus => "res/media/OptimusPrincepsSemiBold.ttf",
	// })
	// .unwrap();
	let lines = create_caption_data(preset, text).await.unwrap();

	let type_face = skia_safe::Typeface::from_name(
		match preset.font {
			Font::Garamond => "Adobe Garamond Pro",
			Font::_Optimus => "OptimusPrincepsSemiBold",
		},
		match preset.font_weight.as_ref().unwrap_or(&FontWeight::Normal) {
			FontWeight::Normal => skia_safe::FontStyle::normal(),
			FontWeight::_Bold => skia_safe::FontStyle::bold(),
		},
	)
	.unwrap();

	let (w, h) = size_guesstimation(text, &type_face);
	let mut surface =
		skia_safe::surfaces::raster_n32_premul((w as i32, h as i32)).expect("Create surface");
	let canvas = surface.canvas();

	let scale = 1_f32;

	const X_OFFSET: f32 = 0_f32;
	const Y_OFFSET: f32 = 0_f32;
	const SCALE_MODIFIER: f32 = 1_f32;

	// TODO Safety and fonts

	let text_opacity = preset.text_opacity.unwrap_or(0.9);
	let blur_tint = &preset.sheen_tint;
	let blur_size = preset.sheen_size;
	let blur_opacity = preset.sheen_opacity;

	let text_color = &preset.main_color;

	let gradient: Option<Infallible> = None;
	let _gradient_scale = 0.5; // We will use this later

	let x0 = (X_OFFSET * w) + w / 2_f32;
	let y0 = (Y_OFFSET * h) + h / 2_f32;
	let scale = scale * SCALE_MODIFIER;

	// Background shade
	canvas.translate((0_f32, y0));
	draw_background(canvas, preset, scale);
	canvas.translate((x0, 0_f32));

	// Text
	canvas.save();

	let zoom_steps = f32::floor(20_f32 * blur_size * f32::powf(scale, 4_f32.recip())) as i32;
	const VERTICAL_OFFSET_MOD: f32 = 1_f32;
	let vertical_offset = VERTICAL_OFFSET_MOD * scale / (blur_size - 1_f32);

	let fill_style = if gradient.is_some() {
		// Create gradient
		skia_safe::Paint::default()
	} else {
		let color = text_color * blur_tint;
		skia_safe::Paint::new(
			<&Rgb as std::convert::Into<skia_safe::Color4f>>::into(&color),
			None,
		)
	};

	// DEBUG
	// let zoom_steps = -1;

	// TODO skip if zoom_steps is negative
	for i in (1..=zoom_steps).rev() {
		canvas.save();

		let scale_factor = f32::powf(blur_size, i as f32 / zoom_steps as f32);

		if i != 0 {
			canvas.scale((scale_factor, scale_factor * Y_SCALE));
		}

		let fat_product = f32::powf(scale_factor, f32::log2(blur_size).recip());
		let sigma = f32::floor(scale * f32::powf(scale_factor, 4_f32));
		let blur = skia_safe::image_filters::blur((sigma, sigma), None, None, None);
		let alpha = blur_opacity / fat_product;

		let mut paint = fill_style.clone();

		let paint = if blur.is_some() {
			paint.set_image_filter(blur.unwrap())
		} else {
			&mut paint
		};

		paint.set_alpha_f(alpha);

		draw_caption(
			canvas,
			&lines,
			&type_face,
			vertical_offset * (scale_factor - 1_f32) / Y_SCALE,
			preset.text_spacing,
			paint,
		);

		// Draw text
		canvas.restore();
	}

	canvas.restore();

	// Gradient
	let fill_style = if gradient.is_some() {
		// Create gradient
		skia_safe::Paint::default()
	} else {
		let mut color: skia_safe::Color4f = text_color.into();
		color.a = text_opacity;
		skia_safe::Paint::new(color, None)
	};

	// Draw text again
	canvas.save();
	canvas.scale((1_f32, Y_SCALE));
	draw_caption(
		canvas,
		&lines,
		&type_face,
		0_f32,
		preset.text_spacing,
		&fill_style,
	);
	canvas.restore();

	let encoding = skia_safe::EncodedImageFormat::PNG;
	Ok(surface
		.image_snapshot()
		.encode(None, encoding, Some(100))
		.unwrap())
}

async fn create_caption_data(_preset: &Preset, text: &str) -> Option<CaptionData> {
	use crate::util;

	let lines = text.split('\n').map(|s| s.trim());
	let mut res: Vec<Vec<TempConversion>> = vec![];

	enum Separation {
		String(String),
		UnicodeEmoji(String),
		DiscordEmoji(String),
	}

	struct TempConversion {
		original: Separation,
		converted: Option<LinePart>,
	}

	impl TempConversion {
		fn new(sep: Separation) -> TempConversion {
			TempConversion {
				original: sep,
				converted: None,
			}
		}

		fn string(string: &str) -> TempConversion {
			Self::new(Separation::String(string.to_string()))
		}

		fn unicode_emoji(string: &str) -> TempConversion {
			Self::new(Separation::UnicodeEmoji(string.to_string()))
		}

		fn discord_emoji(string: &str) -> TempConversion {
			Self::new(Separation::DiscordEmoji(string.to_string()))
		}

		async fn convert(&mut self) {
			use data::config;

			async fn url_to_filesystem(
				url: &str,
				base: &str,
				filename: &str,
			) -> std::path::PathBuf {
				let path = path::Path::new(config::DATA_PATH)
					.join(config::SAVED_DIR)
					.join(base)
					.join(filename);

				if path.exists() {
					return path;
				}

				let request = reqwest::get(url).await;
				let data = match request {
					Ok(res) => {
						if !res.status().is_success() {
							logger::error(&format!("Request for {} failed: {:?}", url, res));
							return path::Path::new(config::DATA_PATH)
								.join(config::MEDIA_DIR)
								.join(config::FALLBACK_IMAGE);
						}
						match res.bytes().await {
							Ok(data) => data,
							Err(e) => {
								logger::error(&format!(
                "Could not convert data from {} to bytes: {}. Resorting to fallback",
                url, e
              ));
								return path::Path::new(config::DATA_PATH)
									.join(config::MEDIA_DIR)
									.join(config::FALLBACK_IMAGE);
							}
						}
					}
					Err(e) => {
						logger::error(&format!(
							"Could not find {}: {}. Resorting to fallback",
							url, e
						));
						return path::Path::new(config::DATA_PATH)
							.join(config::MEDIA_DIR)
							.join(config::FALLBACK_IMAGE);
					}
				};

				let parent = match path.parent() {
					Some(parent) => parent,
					None => {
						logger::error(&format!("{:?} has no parent", path));
						return path::Path::new(config::DATA_PATH)
							.join(config::MEDIA_DIR)
							.join(config::FALLBACK_IMAGE);
					}
				};

				match fs::create_dir_all(parent) {
					Ok(()) => (),
					Err(e) => {
						logger::error(&format!(
							"Could not create directories in {:?}: {}",
							parent, e
						));
						return path::Path::new(config::DATA_PATH)
							.join(config::MEDIA_DIR)
							.join(config::FALLBACK_IMAGE);
					}
				}
				match tokio::fs::write(&path, data).await {
					Ok(_) => path,
					Err(e) => {
						logger::error(&format!(
							"Could not write data to {}: {}. Resorting to fallback",
							path.display(),
							e
						));
						path::Path::new(config::DATA_PATH)
							.join(config::MEDIA_DIR)
							.join(config::FALLBACK_IMAGE)
					}
				}
			}

			match self.original {
				Separation::String(ref string) => {
					self.converted = Some(LinePart::String(string.to_uppercase()))
				}
				Separation::UnicodeEmoji(ref emoji) => {
					let filename = util::filename_from_unicode_emoji(emoji);
					let url = util::url_from_unicode_emoji(emoji);
					self.converted = Some(LinePart::Image(
						url_to_filesystem(&url, "unicode", &filename).await,
						true,
					));
				}
				Separation::DiscordEmoji(ref emoji) => {
					let regex_match = util::DISCORD_EMOJI_REGEX
						.captures(emoji)
						.expect("Emoji was not a match?");
					let animated = !regex_match.get(1).unwrap().is_empty();
					let name = regex_match.get(2).unwrap().as_str();
					let id = regex_match
						.get(3)
						.unwrap()
						.as_str()
						.parse::<u64>()
						.expect("id was not numeric?");

					let filename =
						format!("{}-{}", name, filename_from_discord_emoji(id, animated));
					let url = util::url_from_discord_emoji(id, animated);
					self.converted = Some(LinePart::Image(
						url_to_filesystem(&url, "discord", &filename).await,
						false,
					));
				}
			}
		}
	}

	for line in lines.into_iter() {
		let parts: Vec<TempConversion> = line
			.match_map(&util::EMOJI_REGEX, |(string, is_match)| {
				if is_match {
					match string.chars().next().unwrap() {
						'0'..='9' => {
							vec![TempConversion::string(string)]
						}
						'©' => vec![TempConversion::string(string)],
						'®' => vec![TempConversion::string(string)],
						'™' => vec![TempConversion::string(string)],
						_ => vec![TempConversion::unicode_emoji(string)],
					}
				} else {
					string
						.match_map(&util::DISCORD_EMOJI_REGEX, |(string, is_match)| {
							if is_match {
								TempConversion::discord_emoji(string)
							} else {
								TempConversion::string(string)
							}
						})
						.collect()
				}
			})
			.flatten()
			.collect();

		res.push(parts);
	}

	futures::future::join_all(
		res.iter_mut()
			.map(|l| futures::future::join_all(l.iter_mut().map(|p| p.convert()))),
	)
	.await;

	// Load and save images
	Some(
		res.into_iter()
			.map(|l| {
				l.into_iter()
					.map(|p| p.converted.expect("Should have been Some"))
					.collect::<Vec<LinePart>>()
			})
			.collect(),
	)
}

fn draw_background(canvas: &mut skia_safe::Canvas, preset: &Preset, scale: f32) {
	let w = unsafe { canvas.surface() }.unwrap().width() as f32;
	let h = unsafe { canvas.surface().unwrap() }.height() as f32;

	const SHADOW_SIZE: f32 = 1_f32;
	const SHADOW_OFFSET: f32 = 0_f32;
	const SHADOW_SOFTNESS: f32 = 1_f32;

	let shadow_opacity = preset.shadow_opacity.unwrap_or(0.7_f32);

	let shadow_height = SHADOW_SIZE * 0.95_f32 * h * scale;
	let shadow_center = SHADOW_OFFSET * scale * h;
	let top = shadow_center - shadow_height / 2_f32;
	let bottom = shadow_center + shadow_height / 2_f32;

	let softness_low = f32::min(1_f32, SHADOW_SOFTNESS);
	let softness_high = f32::max(1_f32, SHADOW_SOFTNESS) - 1_f32;

	use skia_safe::Color;

	let colors = &[
		Color::new(0),
		Color::from_argb((shadow_opacity * 255_f32).floor() as u8, 0, 0, 0),
		Color::from_argb((shadow_opacity * 255_f32).floor() as u8, 0, 0, 0),
		Color::new(0),
	];
	let points = &[
		(0_f32),
		(0.25 * softness_low),
		((1_f32 - 0.25) * softness_low),
		(1_f32),
	];

	let gradient_colors = skia_safe::gradient_shader::GradientShaderColors::Colors(&colors[..]);
	let gradient = skia_safe::gradient_shader::linear(
		((w / 2_f32, top), (w / 2_f32, bottom)),
		gradient_colors,
		Some(&points[..]),
		skia_safe::TileMode::Clamp,
		None,
		None,
	)
	.unwrap();

	let sigma = if softness_high > 0_f32 {
		f32::floor(shadow_height * softness_high / 4_f32)
	} else {
		0_f32
	};

	canvas.draw_rect(
		skia_safe::Rect::new(0_f32, top, w, top + shadow_height),
		skia_safe::Paint::new(skia_safe::colors::BLACK, None)
			.set_shader(gradient)
			.set_image_filter(skia_safe::image_filters::blur(
				(sigma, sigma),
				None,
				None,
				None,
			)),
	);
}

fn draw_caption(
	canvas: &mut skia_safe::Canvas,
	lines: &CaptionData,
	type_face: &skia_safe::Typeface,
	y_offset: f32,
	letter_spacing: f32,
	paint: &skia_safe::Paint,
) {
	// TODO Scale to text canvas.scale((1200_f32, 280_f32))

	let letter_spacing = letter_spacing / 4_f32;

	canvas.save();

	let font = skia_safe::Font::new(type_face, Some(FONT_SIZE as f32));

	enum DrawData {
		TextBlob {
			data: skia_safe::TextBlob,
			width: f32,
			height: f32,
		},
		Image {
			data: skia_safe::Image,
			width: f32,
			height: f32,
		},
	}

	impl DrawData {
		fn width(&self) -> f32 {
			match self {
				DrawData::TextBlob { width, .. } => *width,
				DrawData::Image { width, .. } => *width,
			}
		}

		fn _height(&self) -> f32 {
			match self {
				DrawData::TextBlob { height, .. } => *height,
				DrawData::Image { height, .. } => *height,
			}
		}
	}

	let (_, metrics) = font.metrics();
	let line_amount = lines.len();

	let mut line_heights: Vec<f32> = vec![];
	line_heights.reserve(line_amount);

	let mut max_height = 0_f32;
	let mut total_height = 0_f32;

	let mut max_width = 0_f32;

	let draw_data: Vec<Vec<DrawData>> = lines
		.iter()
		.map(|line| {
			let mut max_line_height = 0_f32;
			let mut line_width = 0_f32;
			let res = line
				.iter()
				.map(|part| match part {
					LinePart::String(text) => {
						let mut glyphs = vec![0_u16; text.chars().count()];
						type_face.str_to_glyphs(text, glyphs.as_mut_slice());
						let mut widths = vec![0_f32; glyphs.len()];
						font.get_widths(&glyphs, widths.as_mut_slice());

						let mut cumulative_widths = vec![];
						cumulative_widths.reserve(text.len());

						let mut cumulative: f32 = letter_spacing;
						for width in widths.iter() {
							cumulative_widths.push(cumulative);
							cumulative += width + letter_spacing;
						}

						let height = metrics.cap_height;
						max_line_height = max_line_height.max(height);

						let text = skia_safe::TextBlob::from_pos_text_h(
							text.as_bytes(),
							cumulative_widths.as_slice(),
							0_f32,
							&font,
							None,
						)
						.unwrap();

						line_width += cumulative;

						DrawData::TextBlob {
							data: text,
							width: cumulative,
							height,
						}
					}
					LinePart::Image(src, unicode) => {
						let data = fs::read(src).expect("exists");
						let data = skia_safe::Data::new_copy(data.as_slice());
						let image = skia_safe::Image::from_encoded(data).expect("Is valid");

						let image_height = if *unicode {
							128_f32
						} else {
							f32::min(128_f32, image.height() as f32)
						};
						let image_width = if *unicode {
							128_f32
						} else {
							f32::min(128_f32, image.width() as f32)
						};

						max_height = max_height.max(image_height / Y_SCALE);
						max_line_height = max_line_height.max(image_height / Y_SCALE);

						line_width += image_width;

						DrawData::Image {
							width: image_width,
							height: image_height,
							data: image,
						}
					}
				})
				.collect();

			line_heights.push(max_line_height);
			total_height += max_line_height;
			max_width = max_width.max(line_width);

			res
		})
		.collect();

	total_height += (5 * line_amount - 1) as f32;
	let average_line_height = total_height / line_amount as f32;
	let canvas_width = unsafe { canvas.surface().unwrap().width() } as f32;

	if max_width * 1.1_f32 > canvas_width {
		canvas.scale((
			canvas_width / (max_width * 1.1_f32),
			canvas_width / (max_width * 1.1_f32),
		));
	}

	let top = -total_height / 2_f32;

	draw_data.into_iter().enumerate().for_each(|(i, line)| {
		let line_width: f32 = line.iter().map(DrawData::width).sum();
		let x0 = -line_width / 2_f32;
		let mut x = x0;
		let y0 = top + y_offset;
		let y = y0 + (average_line_height * i as f32);

		line.into_iter().for_each(|part| match part {
			DrawData::TextBlob {
				data,
				width,
				height,
			} => {
				let top = y + (average_line_height - height) / 2_f32;
				canvas.draw_text_blob(data, (x, top + height), paint);

				x += width;
			}
			DrawData::Image {
				data,
				width,
				height,
			} => {
				let top = y + (average_line_height - height / Y_SCALE) / 2_f32;
				canvas.draw_image_rect_with_sampling_options(
					data,
					None,
					skia_safe::Rect::new(x, top, x + width, top + height / Y_SCALE),
					skia_safe::SamplingOptions::new(
						skia_safe::FilterMode::Linear,
						skia_safe::MipmapMode::Linear,
					),
					paint,
				);
				x += width;
			}
		});
	});

	canvas.restore();
}

#[tokio::test]
async fn test_banner() -> Result<(), &'static str> {
	use crate::util::logger;
	use std::env;
	env::set_current_dir("/mnt/lnxdata/data/code/sirgovan-rust/").unwrap();
	logger::debug("Start");
	let content = create_image("PAY OUT THE BELIEVERS 😎", &Preset::BONFIRE_LIT).await?;
	logger::debug("End");
	fs::write("res/tmp.png", content.as_bytes()).unwrap();

	Ok(())
}
