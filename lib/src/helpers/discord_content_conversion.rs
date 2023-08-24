use serenity::model::prelude::*;

pub enum ContentOriginal {
	User(UserId),
	Channel(ChannelId),
	Role(RoleId),
	Emoji(EmojiId),
}

#[allow(dead_code)]
impl ContentOriginal {
	pub fn id(&self) -> u64 {
		match self {
			Self::User(id) => id.0,
			Self::Channel(id) => id.0,
			Self::Role(id) => id.0,
			Self::Emoji(id) => id.0,
		}
	}
}

#[derive(thiserror::Error, Debug)]
pub enum ConversionError {
	#[error("not enough elements supplied")]
	NotEnoughElements,
}

pub struct ContentConverter {
	content: String,
	user: bool,
	channel: bool,
	role: bool,
	emoji: bool,
}

impl ContentConverter {
	const MARKER_START: char = '\x01';
	const MARKER_END: char = '\x02';

	pub fn new(str: String) -> ContentConverter {
		ContentConverter {
			content: str,
			user: false,
			channel: false,
			role: false,
			emoji: false,
		}
	}

	pub fn user(mut self) -> ContentConverter {
		self.user = true;
		self
	}

	pub fn channel(mut self) -> ContentConverter {
		self.channel = true;
		self
	}

	pub fn role(mut self) -> ContentConverter {
		self.role = true;
		self
	}

	pub fn emoji(mut self) -> ContentConverter {
		self.emoji = true;
		self
	}

	pub fn take(&mut self) -> anyhow::Result<Vec<ContentOriginal>> {
		use std::fmt::Write;

		let mut new = String::with_capacity(self.content.len());
		let mut res: Vec<ContentOriginal> = Vec::new();

		let mut chars = StringSearch::new(&self.content);

		fn consume_id(chars: &mut StringSearch) -> Option<u64> {
			let start = chars.as_str();
			let mut end = 0;

			while let Some('0'..='9') = chars.peek() {
				end += 1;
				chars.next();
			}

			let c = chars.peek();

			match c {
				Some('>') => {
					chars.next();
				}
				_ => return None,
			}

			let maybe_id = &start[..end];
			maybe_id.parse::<u64>().ok()
		}

		fn consume_emoji_name(chars: &mut StringSearch) -> bool {
			while chars.peek().is_some_and(|c| c != ':') {
				chars.next();
			}

			matches!(chars.next(), Some(':'))
		}

		loop {
			let pre_loop = chars.get();
			let Some(c) = chars.next() else { break };

			if c == '<' {
				let Some(p) = chars.next() else {
          break;
        };
				match p {
					'@' if self.role || self.user => match chars.peek() {
						Some('&') if self.role => {
							chars.next();
							if let Some(id) = consume_id(&mut chars) {
								res.push(ContentOriginal::Role(id.into()));

								new.push_str(pre_loop);
								chars.update();

								new.push(Self::MARKER_START);
								write!(new, "{}", res.len() - 1)?;
								new.push(Self::MARKER_END);
							}
						}
						_ if self.user => {
							if let Some(id) = consume_id(&mut chars) {
								res.push(ContentOriginal::User(id.into()));

								new.push_str(pre_loop);
								chars.update();

								new.push(Self::MARKER_START);
								write!(new, "{}", res.len() - 1)?;
								new.push(Self::MARKER_END);
							}
						}
						_ => (),
					},
					'#' if self.channel => {
						if let Some(id) = consume_id(&mut chars) {
							res.push(ContentOriginal::Channel(id.into()));

							new.push_str(pre_loop);
							chars.update();

							new.push(Self::MARKER_START);
							write!(new, "{}", res.len() - 1)?;
							new.push(Self::MARKER_END);
						}
					}
					'a' if self.emoji => {
						if let Some(':') = chars.peek() {
							chars.next();
							if consume_emoji_name(&mut chars) {
								if let Some(id) = consume_id(&mut chars) {
									res.push(ContentOriginal::Emoji(id.into()));

									new.push_str(pre_loop);
									chars.update();

									new.push(Self::MARKER_START);
									write!(new, "{}", res.len() - 1)?;
									new.push(Self::MARKER_END);
								}
							}
						}
					}
					':' if self.emoji => {
						if consume_emoji_name(&mut chars) {
							if let Some(id) = consume_id(&mut chars) {
								res.push(ContentOriginal::Emoji(id.into()));

								new.push_str(pre_loop);
								chars.update();

								new.push(Self::MARKER_START);
								write!(new, "{}", res.len() - 1)?;
								new.push(Self::MARKER_END);
							}
						}
					}
					_ => (),
				}
			}
		}

		new.push_str(chars.get());

		self.content = new;

		Ok(res)
	}

	pub fn transform<T: FnOnce(String) -> String>(&mut self, f: T) {
		let tmp = std::mem::take(&mut self.content);
		self.content = f(tmp);
	}

	pub fn replace(&mut self, elems: &[String]) -> anyhow::Result<()> {
		let mut new = String::with_capacity(
			self.content.len() + elems.iter().map(|e| e.len()).sum::<usize>(),
		);

		let mut chars = self.content.chars();

		loop {
			let Some(c) = chars.next() else { break; };
			if c == Self::MARKER_START {
				let num = chars.as_str();
				let mut i = 0;
				while let Some('0'..='9') = chars.next() {
					i += 1;
				}
				let index = num[..i].parse::<usize>()?;
				let replacement = elems.get(index).ok_or(ConversionError::NotEnoughElements)?;
				new.push_str(replacement);
			} else {
				new.push(c);
			}
		}

		self.content = new;

		Ok(())
	}

	pub fn finish(self) -> String {
		self.content
	}
}

struct StringSearch<'a> {
	string: &'a str,
	iter: std::iter::Peekable<std::str::CharIndices<'a>>,
	start: usize,
	end: usize,
}

impl<'a> StringSearch<'a> {
	pub fn new(content: &str) -> StringSearch {
		StringSearch {
			string: content,
			iter: content.char_indices().peekable(),
			start: 0,
			end: 0,
		}
	}

	pub fn get(&self) -> &'a str {
		&self.string[self.start..self.end]
	}

	pub fn peek(&mut self) -> Option<char> {
		Some(self.iter.peek()?.1)
	}

	pub fn as_str(&self) -> &'a str {
		&self.string[self.end..]
	}

	pub fn update(&mut self) {
		self.start = self.end;
	}
}

impl<'a> Iterator for StringSearch<'a> {
	type Item = char;

	fn next(&mut self) -> Option<Self::Item> {
		let (_, e) = self.iter.next()?;
		self.end += e.len_utf8();
		Some(e)
	}
}

#[test]
fn it_works() -> anyhow::Result<()> {
	let start = "<@1><@&2><#3><:b:4><a:c:5>".to_string();
	let end = "12345";

	let mut converter = ContentConverter::new(start).user().channel().emoji().role();

	let ids = converter.take()?;
	let ids = ids
		.into_iter()
		.map(|id| id.id().to_string())
		.collect::<Vec<_>>();
	converter.replace(&ids)?;

	assert_eq!(&converter.finish(), end);

	Ok(())
}
