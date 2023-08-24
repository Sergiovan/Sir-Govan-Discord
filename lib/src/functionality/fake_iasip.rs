use crate::{helpers::handlebars::AlwaysSunnyData, prelude::*};
use image::EncodableLayout;
use serenity::model::prelude::*;
use serenity::prelude::*;

use std::convert::Infallible;

use crate::bot::Bot;

impl Bot {
	pub async fn maybe_iasip(
		&self,
		ctx: Context,
		msg: Message,
		reaction: Reaction,
	) -> Option<Infallible> {
		let channel = msg
			.channel(&ctx)
			.await
			.ok_or_log("Could not fetch message channels")?
			.guild()
			.log_if_none("Message was not in guild")?;

		// Clean content
		let content = msg.content_safe(&ctx); // TODO Images and proper cleaning
									  // Pick song name
		let song_name = std::path::Path::new(data::config::RESOURCE_PATH)
			.join(data::config::MEDIA_DIR)
			.join("tempsens.ogg"); // TODO Put tempsens in data::config
					   // Pick show name
		let show_name = "It's Always Sunny in Here".to_string(); // TODO Proper name pick

		let video = {
			let tmpdir = tempdir::TempDir::new("video").ok()?;
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

			std::fs::write(&concat_file_path, concat_file).ok()?;

			{
				let screenshotter = self.get_screenshotter().await;
				let screenshotter = screenshotter.as_ref().unwrap(); // TODO error checks

				let episode = screenshotter
					.always_sunny(AlwaysSunnyData { text: content })
					.await
					.ok()?;
				let title = screenshotter
					.always_sunny(AlwaysSunnyData { text: show_name })
					.await
					.ok()?;

				std::fs::write(&episode_image, episode).ok()?;
				std::fs::write(&title_image, title).ok()?;
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

			cmd.spawn().ok()?.wait().await.ok()?;

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

			cmd.spawn().ok()?.wait().await.ok()?;

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

			cmd.spawn().ok()?.wait().await.ok()?;

			std::fs::read(&final_output).ok()?
		};

		channel
			.send_message(&ctx, |b| {
				b.reference_message(&msg)
					.allowed_mentions(|b| b.empty_users())
					.add_file(AttachmentType::Bytes {
						data: std::borrow::Cow::Borrowed(video.as_bytes()),
						filename: "iasip.mp4".to_string(),
					})
			})
			.await
			.ok()?;

		return None;
	}
}
