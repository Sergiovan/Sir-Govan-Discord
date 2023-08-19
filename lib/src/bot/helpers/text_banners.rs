use std::fs;

use crate::util::{MatchMap, OptionErrorHandler, ResultErrorHandler};

use skia_safe;

use std::ops::Mul;
use std::{mem, path};

use lazy_static::lazy_static;

#[derive(Clone)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
	pub fn into_color4f(self, opacity: Option<f32>) -> skia_safe::Color4f {
		skia_safe::Color4f::new(
			self.0 as f32 / 255_f32,
			self.1 as f32 / 255_f32,
			self.2 as f32 / 255_f32,
			opacity.unwrap_or(1_f32),
		)
	}
}

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

#[derive(Clone)]
pub enum Font {
	Garamond,
	Optimus,
}

#[derive(Clone)]
pub enum FontWeight {
	Normal,
	Bold,
}

#[derive(Clone)]
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
	pub const HUMANITY_RESTORED: Preset = Preset {
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

	pub const VICTORY_ACHIEVED: Preset = Preset {
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

	pub const YOU_DIED: Preset = Preset {
		main_color: Rgb(101, 5, 4),
		sheen_tint: Rgb(0, 0, 0),

		text_spacing: 8_f32,
		sheen_size: 0.0,
		sheen_opacity: 0.0,

		text_opacity: Some(1.0),
		shadow_opacity: Some(1.0),

		font: Font::Optimus,
		font_weight: Some(FontWeight::Bold),
	};
}

pub type Gradient = [Rgb];

pub mod gradients {
	use super::Gradient;
	use super::Rgb;

	// Govan is WOKE
	pub const LGBT: &Gradient = &[
		Rgb(0xff, 0x00, 0x00),
		Rgb(0xff, 0x88, 0x00),
		Rgb(0xff, 0xee, 0x00),
		Rgb(0x00, 0xaa, 0x00),
		Rgb(0x22, 0x66, 0xcc),
		Rgb(0xaa, 0x00, 0xaa),
	];

	pub const TRANS: &Gradient = &[
		Rgb(0x77, 0xbb, 0xff),
		Rgb(0x77, 0xbb, 0xff),
		Rgb(0xff, 0x99, 0xaa),
		Rgb(0xff, 0x99, 0xaa),
		Rgb(0xff, 0xff, 0xff),
		Rgb(0xff, 0xff, 0xff),
		Rgb(0xff, 0x99, 0xaa),
		Rgb(0xff, 0x99, 0xaa),
		Rgb(0x77, 0xbb, 0xff),
		Rgb(0x77, 0xbb, 0xff),
	];

	pub const BI: &Gradient = &[
		Rgb(0xff, 0x00, 0x88),
		Rgb(0xff, 0x00, 0x88),
		Rgb(0xaa, 0x66, 0xaa),
		Rgb(0x88, 0x00, 0xff),
		Rgb(0x88, 0x00, 0xff),
	];

	pub const LESBIAN: &Gradient = &[
		Rgb(0xff, 0x22, 0x00),
		Rgb(0xff, 0x66, 0x44),
		Rgb(0xff, 0xaa, 0x88),
		Rgb(0xff, 0xff, 0xff),
		Rgb(0xff, 0x88, 0xff),
		Rgb(0xff, 0x44, 0xcc),
		Rgb(0xff, 0x00, 0x88),
	];

	pub const ENBI: &Gradient = &[
		Rgb(0xff, 0xff, 0x22),
		Rgb(0xff, 0xff, 0xff),
		Rgb(0x88, 0x44, 0xdd),
		Rgb(0x33, 0x33, 0x33),
	];

	pub const PAN: &Gradient = &[
		Rgb(0xff, 0x22, 0xcc),
		Rgb(0xff, 0x22, 0xcc),
		Rgb(0xff, 0xff, 0x22),
		Rgb(0xff, 0xff, 0x22),
		Rgb(0x22, 0xcc, 0xff),
		Rgb(0x22, 0xcc, 0xff),
	];
}

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

struct LineData {
	data: Vec<DrawData>,
	width: f32,
	_height: f32,
}

struct CaptionData {
	lines: Vec<LineData>,
	width: f32,
	height: f32,
}

#[derive(Debug)]
enum LineElement {
	String(String),
	Image(path::PathBuf, bool),
}

const Y_SCALE: f32 = 1.5_f32;
const FONT_SIZE: f32 = 92_f32;

pub async fn create_image(
	text: &str,
	preset: &Preset,
	gradient: Option<&Gradient>,
) -> Result<skia_safe::Data, &'static str> {
	let type_face = skia_safe::Typeface::from_name(
		match preset.font {
			Font::Garamond => "Adobe Garamond Pro",
			Font::Optimus => "OptimusPrincepsSemiBold",
		},
		match preset.font_weight.as_ref().unwrap_or(&FontWeight::Normal) {
			FontWeight::Normal => skia_safe::FontStyle::normal(),
			FontWeight::Bold => skia_safe::FontStyle::bold(),
		},
	)
	.unwrap();
	let font = skia_safe::Font::new(&type_face, Some(FONT_SIZE));

	let lines = create_caption_data(preset, text, &type_face, &font)
		.await
		.unwrap();

	let (w, h) = (lines.width.mul(1.1_f32).clamp(1200_f32, 1920_f32), 280_f32);
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

	let x0 = (X_OFFSET * w) + w / 2_f32;
	let y0 = (Y_OFFSET * h) + h / 2_f32;
	let scale = scale * SCALE_MODIFIER;

	// Background shade
	canvas.translate((0_f32, y0));
	draw_background(canvas, (w, h), preset, scale);
	canvas.translate((x0, 0_f32));

	// Text
	canvas.save();

	let zoom_steps = f32::floor(20_f32 * blur_size * f32::powf(scale, 4_f32.recip())) as i32;
	const VERTICAL_OFFSET_MOD: f32 = 1_f32;
	let vertical_offset = VERTICAL_OFFSET_MOD * scale / (blur_size - 1_f32);

	let fill_style = if gradient.is_some() {
		let gradient = create_gradient(gradient.unwrap(), w, Some(1_f32), Some(blur_tint));

		skia_safe::Paint::default().set_shader(gradient).clone()
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
			w,
			&lines,
			vertical_offset * (scale_factor - 1_f32) / Y_SCALE,
			paint,
		);

		// Draw text
		canvas.restore();
	}

	canvas.restore();

	// Gradient
	let fill_style = if gradient.is_some() {
		let gradient = create_gradient(gradient.unwrap(), w, Some(text_opacity), None);

		skia_safe::Paint::default().set_shader(gradient).clone()
	} else {
		let mut color: skia_safe::Color4f = text_color.into();
		color.a = text_opacity;
		skia_safe::Paint::new(color, None)
	};

	// Draw text again
	canvas.save();
	canvas.scale((1_f32, Y_SCALE));
	draw_caption(canvas, w, &lines, 0_f32, &fill_style);
	canvas.restore();

	let encoding = skia_safe::EncodedImageFormat::PNG;
	Ok(
		surface
			.image_snapshot()
			.encode(None, encoding, Some(100))
			.unwrap(),
	)
}

async fn create_caption_data(
	preset: &Preset,
	text: &str,
	type_face: &skia_safe::Typeface,
	font: &skia_safe::Font,
) -> Option<CaptionData> {
	use crate::util;

	let letter_spacing = preset.text_spacing / 4_f32;

	let lines = text.split('\n').map(|s| s.trim());
	let mut res: Vec<Vec<AsyncElementConverter>> = vec![];

	#[derive(Debug)]
	enum LineElementRaw {
		String(String),
		UnicodeEmoji(String),
		DiscordEmoji(String),
	}

	impl LineElementRaw {
		async fn convert(self) -> LineElement {
			use crate::bot::data::config;

			async fn url_to_filesystem(url: &str, base: &str, filename: &str) -> Option<std::path::PathBuf> {
				let path = path::Path::new(config::DATA_PATH)
					.join(config::SAVED_DIR)
					.join(base)
					.join(filename);

				if path.exists() {
					return Some(path);
				}

				let request = reqwest::get(url).await;
				let res = request.ok_or_log(&format!("Could not find {}", url))?;
				let data = res
					.bytes()
					.await
					.ok_or_log(&format!("Could not convert data from {} to bytes", url))?;

				let parent = path
					.parent()
					.log_if_none(&format!("{:?} has no parent", path))?;

				fs::create_dir_all(parent).ok_or_log(&format!(
					"Could not create directories in {}",
					parent.display()
				))?;

				use crate::util::logger;

				match image::load_from_memory_with_format(&data, image::ImageFormat::Png) {
					Ok(_) => (),
					Err(e) => {
						logger::error(&format!(
							"Image from {} does not have the correct format: {}",
							url, e
						));
						return None;
					}
				}

				tokio::fs::write(&path, data)
					.await
					.ok_or_log(&format!("Could not write data to {}", path.display()))?;

				Some(path)
			}

			lazy_static! {
				static ref FALLBACK_IMAGE: path::PathBuf = path::Path::new(config::DATA_PATH)
					.join(config::MEDIA_DIR)
					.join(config::FALLBACK_IMAGE);
			}

			match self {
				LineElementRaw::String(string) => LineElement::String(string.to_uppercase()),
				LineElementRaw::UnicodeEmoji(emoji) => {
					let filename = util::filename_from_unicode_emoji(&emoji);
					let url = util::url_from_unicode_emoji(&emoji);
					LineElement::Image(
						url_to_filesystem(&url, "unicode", &filename)
							.await
							.unwrap_or(FALLBACK_IMAGE.clone()),
						true,
					)
				}
				LineElementRaw::DiscordEmoji(emoji) => {
					let regex_match = util::DISCORD_EMOJI_REGEX
						.captures(&emoji)
						.expect("Emoji was not a match?");
					let animated = !regex_match.get(1).unwrap().is_empty();
					let name = regex_match.get(2).unwrap().as_str();
					let id = regex_match
						.get(3)
						.unwrap()
						.as_str()
						.parse::<u64>()
						.expect("id was not numeric?");

					let filename = format!(
						"{}-{}",
						name,
						util::filename_from_discord_emoji(id, animated)
					);
					let url = util::url_from_discord_emoji(id, animated);
					LineElement::Image(
						url_to_filesystem(&url, "discord", &filename)
							.await
							.unwrap_or(FALLBACK_IMAGE.clone()),
						false,
					)
				}
			}
		}
	}

	#[derive(Debug)]
	enum AsyncElementConverter {
		Default,
		Original(LineElementRaw),
		Converted(LineElement),
	}

	impl AsyncElementConverter {
		fn new(sep: LineElementRaw) -> AsyncElementConverter {
			AsyncElementConverter::Original(sep)
		}

		fn string(string: &str) -> AsyncElementConverter {
			Self::new(LineElementRaw::String(string.to_string()))
		}

		fn unicode_emoji(string: &str) -> AsyncElementConverter {
			Self::new(LineElementRaw::UnicodeEmoji(string.to_string()))
		}

		fn discord_emoji(string: &str) -> AsyncElementConverter {
			Self::new(LineElementRaw::DiscordEmoji(string.to_string()))
		}

		async fn convert(&mut self) {
			let tmp = std::mem::replace(self, Self::Default);

			_ = mem::replace(
				self,
				Self::Converted(match tmp {
					Self::Original(r) => r.convert().await,
					Self::Converted(_) => panic!("Called convert() on converted value: {:?}", tmp),
					Self::Default => panic!("Called convert() on a default value: {:?}", tmp),
				}),
			);
		}

		fn take(self) -> LineElement {
			match self {
				Self::Converted(t) => t,
				Self::Original(_) => panic!("Trying to take out of original element: {:?}", self),
				Self::Default => panic!("Trying to take out of defaulted element: {:?}", self),
			}
		}
	}

	for line in lines.into_iter() {
		let parts: Vec<AsyncElementConverter> = line
			.match_map(&util::EMOJI_REGEX, |(string, is_match)| {
				if is_match {
					match string.chars().next().unwrap() {
						'0'..='9' => {
							vec![AsyncElementConverter::string(string)]
						}
						'©' => vec![AsyncElementConverter::string(string)],
						'®' => vec![AsyncElementConverter::string(string)],
						'™' => vec![AsyncElementConverter::string(string)],
						_ => vec![AsyncElementConverter::unicode_emoji(
							string.trim_end_matches('\u{fe0f}'),
						)],
					}
				} else {
					string
						.match_map(&util::DISCORD_EMOJI_REGEX, |(string, is_match)| {
							if is_match {
								AsyncElementConverter::discord_emoji(string)
							} else {
								AsyncElementConverter::string(string)
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
		res
			.iter_mut()
			.map(|l| futures::future::join_all(l.iter_mut().map(|p| p.convert()))),
	)
	.await;

	let (_, metrics) = font.metrics();
	let line_amount = res.len();

	let mut line_heights: Vec<f32> = vec![];
	line_heights.reserve(line_amount);

	let mut max_height = 0_f32;
	let mut total_height = 0_f32;

	let mut max_width = 0_f32;

	// Load and save images
	let caption_lines: Vec<LineData> = res
		.into_iter()
		.map(|l| {
			let mut max_line_height = 0_f32;
			let mut line_width = 0_f32;
			let line_data: Vec<DrawData> = l
				.into_iter()
				.map(|p| {
					let part = p.take();
					match part {
						LineElement::String(text) => {
							let mut glyphs = vec![0_u16; text.chars().count()];
							type_face.str_to_glyphs(&text, glyphs.as_mut_slice());
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
								font,
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
						LineElement::Image(src, unicode) => {
							let data = fs::read(src).expect("exists");
							let data = skia_safe::Data::new_copy(data.as_slice());
							let image = skia_safe::Image::from_encoded(data).expect("Is valid");

							let image_height = if unicode {
								128_f32
							} else {
								f32::min(128_f32, image.height() as f32)
							};
							let image_width = if unicode {
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
					}
				})
				.collect();

			total_height += max_line_height;
			max_width = max_width.max(line_width);

			LineData {
				data: line_data,
				width: line_width,
				_height: max_line_height,
			}
		})
		.collect();

	Some(CaptionData {
		lines: caption_lines,
		width: max_width,
		height: total_height,
	})
}

fn create_gradient(
	gradient: &Gradient,
	width: f32,
	opacity: Option<f32>,
	tint: Option<&Rgb>,
) -> Option<skia_safe::Shader> {
	skia_safe::gradient_shader::linear(
		((-(width / 2_f32), 0_f32), (width / 2_f32, 0_f32)),
		skia_safe::gradient_shader::GradientShaderColors::ColorsInSpace(
			&gradient
				.iter()
				.map(|c| {
					if let Some(tint) = tint {
						(c * tint).into_color4f(opacity)
					} else {
						c.clone().into_color4f(opacity)
					}
				})
				.collect::<Vec<_>>(),
			None,
		),
		Some(
			(0..gradient.len())
				.map(|n| n as f32 / (gradient.len() - 1) as f32)
				.collect::<Vec<_>>()
				.as_slice(),
		),
		skia_safe::TileMode::Repeat,
		None,
		None,
	)
}

fn draw_background(
	canvas: &mut skia_safe::Canvas,
	canvas_size: (f32, f32),
	preset: &Preset,
	scale: f32,
) {
	let (w, h) = canvas_size;

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
	canvas_width: f32,
	caption_data: &CaptionData,
	y_offset: f32,
	paint: &skia_safe::Paint,
) {
	canvas.save();

	let line_amount = caption_data.lines.len();
	let total_height = caption_data.height + (5 * line_amount - 1) as f32;

	let average_line_height = total_height / line_amount as f32;

	if caption_data.width > canvas_width {
		canvas.scale((
			canvas_width / (caption_data.width),
			canvas_width / (caption_data.width),
		));
	}

	let top = -total_height / 2_f32;

	caption_data.lines.iter().enumerate().for_each(|(i, line)| {
		let x0 = -line.width / 2_f32;
		let mut x = x0;
		let y0 = top + y_offset;
		let y = y0 + (average_line_height * i as f32);

		line.data.iter().for_each(|part| match part {
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
					skia_safe::SamplingOptions::new(skia_safe::FilterMode::Linear, skia_safe::MipmapMode::Linear),
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
	let content = create_image(
		"PAY OUT THE BELIEVERS",
		&Preset::BONFIRE_LIT,
		Some(self::gradients::TRANS),
	)
	.await?;
	logger::debug("End");
	fs::write("res/tmp.png", content.as_bytes()).unwrap();

	Ok(())
}
