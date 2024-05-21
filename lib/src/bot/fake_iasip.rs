use crate::util::error::{self, GovanResult};
use crate::{helpers::handlebars::AlwaysSunnyData, prelude::*};

use image::EncodableLayout;
use serenity::builder::{CreateAllowedMentions, CreateAttachment, CreateMessage};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::helpers::discord_content_conversion::{ContentConverter, ContentOriginal};

use crate::bot::Bot;

impl Bot {
	pub async fn maybe_iasip(&self, ctx: &Context, msg: &Message) -> GovanResult {
		async fn stringify_content(ctx: &Context, content: ContentOriginal) -> String {
			match content {
				ContentOriginal::User(id) => format!(
					"@{}",
					id.to_user(&ctx)
						.await
						.map_or("Unknown User".to_string(), |u| u.name)
				),
				ContentOriginal::Channel(id) => format!(
					"#{}",
					id.to_channel(&ctx)
						.await
						.map_or("Unknown Channel".to_string(), |c| c
							.guild()
							.map_or("Unknown Channel".to_string(), |c| c.name))
				),
				ContentOriginal::Role(id) => format!(
					"@{}",
					util::role_from_id(id, ctx)
						.map_or("@Unknown Role".to_string(), |role| role.name)
				),
				ContentOriginal::Emoji(id) => format!(
					r#"<img class="emoji" height="72" width="72" src="{}">"#,
					util::url_from_discord_emoji(id.into(), false)
				),
			}
		}

		let channel = msg
			.channel(&ctx)
			.await?
			.guild()
			.ok_or_else(error::debug_lazy!(
				log = "Not in a guild channel",
				user = "You can only use this inside a guild!"
			))?;

		// Clean content
		let mut converter = ContentConverter::new(msg.content.clone())
			.user()
			.channel()
			.emoji()
			.role();

		let ids = converter.take()?;
		let futures = ids.into_iter().map(|e| stringify_content(ctx, e));
		let replacements = util::collect_async(futures).await;

		let replacements = replacements.into_iter().collect::<Vec<_>>();
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

		let show_name = {
			let strings = &self.data().await.strings;
			if util::random::one_in(10) {
				strings.titlecard_show_entire.pick().clone()
			} else {
				let place_name = if util::random::one_in(5) {
					channel.name.replace('-', " ")
				} else {
					channel
						.guild(ctx)
						.map(|g| g.name.clone())
						.unwrap_or_else(|| channel.name.replace('-', " "))
				};
				let mut chars = place_name.chars();
				let first = chars.next().unwrap();
				format!(
					"{} {}",
					strings.titlecard_show_prefix.pick(),
					first.to_uppercase().chain(chars).collect::<String>()
				)
			}
		};

		let video = {
			let tmpdir = tempfile::TempDir::with_prefix("video")?;

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
				let screenshotter = self.screenshotter().await?;

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

			let mut episode_handle = cmd.spawn()?;

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

			let mut title_handle = cmd.spawn()?;

			let (episode_result, title_result) =
				tokio::join!(episode_handle.wait(), title_handle.wait());

			let episode_result = episode_result?;
			let title_result = title_result?;
			if !episode_result.success() || !title_result.success() {
				let failure = if !episode_result.success() {
					"episode"
				} else {
					"title"
				};
				return Err(error::error!(
					log fmt = (
						"Ffmpeg for {} exited with error {}",
						failure,
						episode_result
							.code()
							.or(title_result.code())
							.unwrap_or(i32::MIN)
					),
					user = "My video editor broke"
				));
			}

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
			.send_message(
				&ctx,
				CreateMessage::default()
					.reference_message(msg)
					.allowed_mentions(CreateAllowedMentions::default().empty_users())
					.add_file(CreateAttachment::bytes(
						std::borrow::Cow::Borrowed(video.as_bytes()),
						"iasip.mp4".to_string(),
					)),
			)
			.await?;

		Ok(())
	}
}
