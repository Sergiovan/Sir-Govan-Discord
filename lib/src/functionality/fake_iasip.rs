use crate::{helpers::handlebars::AlwaysSunnyData, prelude::*};
use image::EncodableLayout;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::helpers::discord_content_conversion::{ContentConverter, ContentOriginal};

use crate::bot::Bot;

#[derive(thiserror::Error, Debug)]
pub enum MaybeIasipError {
	#[error("dicord api error {0}")]
	DiscordError(#[from] serenity::Error),
	#[error("not currently in a guild channel")]
	NotInGuild,
	#[error("generic error {0}")]
	GenericError(#[from] anyhow::Error),
	#[error("error getting ids from message")]
	ConverterError,
	#[error("io error {0}")]
	IoError(#[from] std::io::Error),
}

impl Reportable for MaybeIasipError {
	fn get_messages(&self) -> ReportMsgs {
		let to_logger = Some(self.to_string());
		let to_user: Option<String> = match self {
			Self::DiscordError(..) => Some("Having some trouble with that".into()),
			Self::NotInGuild => Some("You're not in a guild".into()),
			Self::GenericError(..) => {
				Some("Someone sneezed really hard and startled me, sorry".into())
			}
			Self::ConverterError => Some("I am having trouble reading your message".into()),
			Self::IoError(..) => Some("I'm too full of soup for that right now".into()),
		};
		ReportMsgs { to_logger, to_user }
	}
}

impl Bot {
	pub async fn maybe_iasip(&self, ctx: &Context, msg: &Message) -> Result<(), MaybeIasipError> {
		#[derive(thiserror::Error, Debug)]
		enum StringifyError {
			#[error("channel {0} was not guild channel")]
			ChannelNotInGuild(u64),
		}

		async fn stringify_content(
			ctx: &Context,
			content: ContentOriginal,
		) -> anyhow::Result<String> {
			match content {
				ContentOriginal::User(id) => Ok(format!("@{}", id.to_user(&ctx).await?.name)),
				ContentOriginal::Channel(id) => Ok(format!(
					"#{}",
					id.to_channel(&ctx)
						.await?
						.guild()
						.ok_or(StringifyError::ChannelNotInGuild(id.into()))?
						.name
				)),
				ContentOriginal::Role(id) => Ok(format!(
					"@{}",
					id.to_role_cached(ctx)
						.map(|role| role.name.clone())
						.unwrap_or("@Unknown Role".to_string())
				)),
				ContentOriginal::Emoji(id) => Ok(format!(
					r#"<img class="emoji" height="72" width="72" src="{}">"#,
					util::url_from_discord_emoji(id.into(), false)
				)),
			}
		}

		let channel = msg
			.channel(&ctx)
			.await?
			.guild()
			.ok_or(MaybeIasipError::NotInGuild)?;

		// Clean content
		let mut converter = ContentConverter::new(msg.content.clone())
			.user()
			.channel()
			.emoji()
			.role();

		let ids = converter.take()?;
		let futures = ids.into_iter().map(|e| stringify_content(ctx, e));
		let replacements = futures::future::join_all(futures).await;

		if replacements.iter().any(|r| r.is_err()) {
			replacements
				.into_iter()
				.for_each(|r| r.log_if_err("Error finding id"));
			return Err(MaybeIasipError::ConverterError);
		}

		let replacements = replacements
			.into_iter()
			.map(|r| r.unwrap())
			.collect::<Vec<_>>();
		converter.transform(|s| html_escape::encode_safe(&s).to_string());
		converter.replace(&replacements)?;

		let content = converter.finish();
		let content =
			data::regex::EMOJI_REGEX.replace_all(&content, |capture: &regex::Captures| {
				let emoji = capture.get(0).unwrap().as_str();
				match emoji.chars().next().unwrap() {
					'©' => return emoji.to_string(),
					'®' => return emoji.to_string(),
					'™' => return emoji.to_string(),
					_ => (),
				}
				format!(
					r#"<img class="emoji" src="{}">"#,
					util::url_from_unicode_emoji(emoji)
				)
			});
		let content = format!(r#""{content}""#);

		// Pick song name
		let song_name = std::path::Path::new(data::config::RESOURCE_PATH)
			.join(data::config::MEDIA_DIR)
			.join("tempsens.ogg"); // TODO Put tempsens in data::config
					   // Pick show name
		let show_name = "It's Always Sunny in Here".to_string(); // TODO Proper name pick

		let video = {
			let tmpdir = tempdir::TempDir::new("video")?;
			// println!("{}", tmpdir.path().display());

			// TODO Get images, idk
			let episode_image = tmpdir.path().join("episode.png");
			let title_image = tmpdir.path().join("title.png");

			let episode_video = tmpdir.path().join("episode.mp4");
			let title_video = tmpdir.path().join("title.mp4");

			let concat_file = format!(
				"file '{}'\nfile '{}'",
				episode_video.display(),
				title_video.display()
			);

			let concat_file_path = tmpdir.path().join("ffmpeg-concat-files.txt");
			let final_output = tmpdir.path().join("final.mp4");

			std::fs::write(&concat_file_path, concat_file)?;

			{
				let screenshotter = self.get_screenshotter().await;
				let screenshotter = screenshotter.as_ref().unwrap(); // TODO error checks

				let episode = screenshotter
					.always_sunny(AlwaysSunnyData { text: content })
					.await?;
				let title = screenshotter
					.always_sunny(AlwaysSunnyData { text: show_name })
					.await?;

				std::fs::write(&episode_image, episode)?;
				std::fs::write(&title_image, title)?;
			};

			let mut cmd = tokio::process::Command::new("ffmpeg");
			cmd.stdout(std::process::Stdio::null())
				.stderr(std::process::Stdio::null());
			cmd.arg("-loop").arg("1"); // Loop the image
			cmd.arg("-i").arg(episode_image); // Input file
			cmd.arg("-c:v").arg("libx264"); // Codec
			cmd.arg("-t").arg("3"); // Duration of output
			cmd.arg("-preset").arg("ultrafast");
			cmd.arg("-pix_fmt").arg("yuv420p"); // Output pixel format
			cmd.arg("-r").arg("1/3"); // Output framerate (1/3 for optimal speed)
			cmd.arg(&episode_video); // Output

			cmd.spawn()?.wait().await?;

			let mut cmd = tokio::process::Command::new("ffmpeg");
			cmd.stdout(std::process::Stdio::null())
				.stderr(std::process::Stdio::null());
			cmd.arg("-loop").arg("1"); // Loop the image
			cmd.arg("-i").arg(title_image); // Input file
			cmd.arg("-c:v").arg("libx264"); // Codec
			cmd.arg("-t").arg("4"); // Duration of output
			cmd.arg("-preset").arg("ultrafast");
			cmd.arg("-pix_fmt").arg("yuv420p"); // Output pixel format
			cmd.arg("-r").arg("1/4"); // Output framerate (1/3 for optimal speed)
			cmd.arg(&title_video); // Output

			cmd.spawn()?.wait().await?;

			let mut cmd = tokio::process::Command::new("ffmpeg");
			cmd.stdout(std::process::Stdio::null())
				.stderr(std::process::Stdio::null());
			cmd.arg("-f").arg("concat"); // Format: Concat
			cmd.arg("-safe").arg("0"); // Safe?
			cmd.arg("-i").arg(&concat_file_path); // Concat file
			cmd.arg("-i").arg(song_name); // Audio file
			cmd.arg("-c:v").arg("libx264"); // Codec
			cmd.arg("-crf").arg("23"); // crf
			cmd.arg("-profile:v").arg("baseline"); // TODO Figure out
			cmd.arg("-level").arg("3.0");
			cmd.arg("-preset").arg("ultrafast");
			cmd.arg("-pix_fmt").arg("yuv420p");
			cmd.arg("-c:a").arg("aac");
			cmd.arg("-ac").arg("2");
			cmd.arg("-b:a").arg("128k");
			cmd.arg("-movflags").arg("faststart");
			cmd.arg("-r").arg("1"); // 1 fps
			cmd.arg(&final_output);

			cmd.spawn()?.wait().await?;

			std::fs::read(&final_output)?
		};

		channel
			.send_message(&ctx, |b| {
				b.reference_message(msg)
					.allowed_mentions(|b| b.empty_users())
					.add_file(AttachmentType::Bytes {
						data: std::borrow::Cow::Borrowed(video.as_bytes()),
						filename: "iasip.mp4".to_string(),
					})
			})
			.await?;

		Ok(())
	}
}