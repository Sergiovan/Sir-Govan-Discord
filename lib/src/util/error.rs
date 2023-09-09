use super::logger;
use crate::data;
use either::Either::*;
use serenity::{
	model::prelude::{Channel, Message},
	prelude::*,
};

#[derive(Debug, Clone)]
pub enum LogType {
	Debug,
	Warning,
	Error,
}

#[derive(Debug, Clone)]
pub enum RandomMsgType {
	GenericError,
}

#[derive(Debug, Clone)]
pub enum UserMsgType {
	None,
	Normal(String),
	Random(RandomMsgType),
}

pub type GovanResult<T = ()> = std::result::Result<T, GovanError>;

pub struct GovanError(GovanErrorImpl);

impl GovanError {
	pub fn as_debug<A>(to_log: Option<A>, to_user: UserMsgType) -> Self
	where
		A: Into<String>,
	{
		GovanError(GovanErrorImpl::new(
			LogType::Debug,
			to_log.map(A::into),
			to_user,
			None,
		))
	}

	pub fn as_warning<A>(to_log: Option<A>, to_user: UserMsgType) -> Self
	where
		A: Into<String>,
	{
		GovanError(GovanErrorImpl::new(
			LogType::Warning,
			to_log.map(A::into),
			to_user,
			None,
		))
	}

	pub fn as_error<A>(to_log: Option<A>, to_user: UserMsgType) -> Self
	where
		A: Into<String>,
	{
		GovanError(GovanErrorImpl::new(
			LogType::Error,
			to_log.map(A::into),
			to_user,
			None,
		))
	}

	pub fn log(self) {
		self.0.log();
	}

	pub async fn send(
		self,
		ctx: &Context,
		to: either::Either<&Message, &Channel>,
		strings: Option<&data::Strings>,
	) -> Self {
		self.0.send(ctx, to, strings).await;

		self
	}

	pub async fn report(
		self,
		ctx: &Context,
		to: either::Either<&Message, &Channel>,
		strings: Option<&data::Strings>,
	) {
		self.send(ctx, to, strings).await.log()
	}

	pub fn into_err(self) -> anyhow::Error {
		self.0.into()
	}

	pub fn with_log(mut self, log: impl Into<String>) -> Self {
		self.0.to_log = Some(log.into());

		self
	}

	pub fn with_user_string(mut self, user_string: impl Into<String>) -> Self {
		self.0.to_user = UserMsgType::Normal(user_string.into());

		self
	}

	pub fn with_user_string_weak(mut self, user_string: impl Into<String>) -> Self {
		if matches!(self.0.to_user, UserMsgType::None) {
			self.0.to_user = UserMsgType::Normal(user_string.into());
		}

		self
	}

	pub fn without_user_string(mut self) -> Self {
		self.0.to_user = UserMsgType::None;

		self
	}

	pub fn with_source(mut self, error: impl Into<anyhow::Error>) -> Self {
		self.0.source = Some(error.into());

		self
	}

	pub fn map_source<P>(self) -> impl FnOnce(P) -> Self
	where
		P: Into<anyhow::Error>,
	{
		move |e| self.with_source(e.into())
	}
}

impl std::fmt::Display for GovanError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl std::fmt::Debug for GovanError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

macro_rules! into_govan {
	($($ty:ty [$user:expr]),*$(,)?) => {
    $(
      impl From<$ty> for GovanError {
        fn from(value: $ty) -> Self {
          GovanError(GovanErrorImpl {
            log_type: LogType::Error,
            to_log: None,
            to_user: $user,
            source: Some(anyhow::Error::from(value)),
          })
        }
      }
    )*
	};
}

into_govan!(
	SerenityError[UserMsgType::Random(RandomMsgType::GenericError)],
	std::io::Error[UserMsgType::None],
	std::fmt::Error[UserMsgType::None],
	url::ParseError[UserMsgType::None],
	reqwest::Error[UserMsgType::None],
	image::error::ImageError[UserMsgType::None],
	std::num::ParseIntError[UserMsgType::Random(RandomMsgType::GenericError)],
	handlebars::RenderError[UserMsgType::None],
	handlebars::TemplateError[UserMsgType::None],
	fantoccini::error::NewSessionError[UserMsgType::None],
	fantoccini::error::CmdError[UserMsgType::None],
	toml::de::Error[UserMsgType::None],
);

#[derive(thiserror::Error, Debug)]
struct GovanErrorImpl {
	log_type: LogType,
	to_log: Option<String>,
	to_user: UserMsgType,
	#[source]
	source: Option<anyhow::Error>,
}

impl GovanErrorImpl {
	pub fn new(
		log_type: LogType,
		to_log: Option<String>,
		to_user: UserMsgType,
		source: Option<anyhow::Error>,
	) -> Self {
		GovanErrorImpl {
			log_type,
			to_log,
			to_user,
			source,
		}
	}

	pub fn _wrap<A>(
		self,
		log_type: Option<LogType>,
		to_log: Option<A>,
		to_user: UserMsgType,
	) -> Self
	where
		A: Into<String>,
	{
		Self::new(
			log_type.unwrap_or(self.log_type.clone()),
			to_log.map(A::into).or(self.to_log.clone()),
			if matches!(to_user, UserMsgType::None) {
				self.to_user.clone()
			} else {
				to_user
			},
			Some(self.into()),
		)
	}

	async fn send<'a, 'b>(
		&'a self,
		ctx: &'b Context,
		to: either::Either<&'b Message, &'b Channel>,
		strings: Option<&data::Strings>,
	) -> &'a Self {
		let send = |s: String| async {
			if false {
				// Type inference :)
				return Err(error!());
			}

			match to {
				Left(msg) => msg.reply(ctx, s).await.map_err(|e| e.into()),
				Right(channel) => match channel {
					Channel::Guild(guild_channel) => guild_channel
						.send_message(ctx, |b| b.content(s))
						.await
						.map_err(|e| e.into()),
					Channel::Private(private_channel) => private_channel
						.send_message(ctx, |b| b.content(s))
						.await
						.map_err(|e| e.into()),
					_ => Err(error!(
						log fmt = (
						"{:?} is not a valid channel to send a message",
						channel
					))),
				},
			}
		};

		let to_send = match self.to_user {
			UserMsgType::Normal(ref to_user) => {
				if to_user.is_empty() {
					return self;
				}

				to_user.clone()
			}
			UserMsgType::Random(ref r) if strings.is_some() => {
				use crate::util::random::RandomBag;
				let strings = strings.unwrap();

				match r {
					RandomMsgType::GenericError => strings.generic_error.pick().clone(),
				}
			}
			_ => return self,
		};

		let res = send(to_send).await;

		if res.is_err() {
			res.unwrap_err().log();
		}

		self
	}

	fn log(self) {
		if self.to_log.is_some() {
			match self.log_type {
				LogType::Debug => {
					if self.source.is_some() {
						let error_msg = self.source.as_ref().unwrap().to_string();
						if error_msg.is_empty() {
							logger::debug_fmt!("{}", self)
						} else {
							logger::debug_fmt!("{}: {}", self, self.source.as_ref().unwrap())
						}
					} else {
						logger::debug_fmt!("{}", self)
					}
				}
				LogType::Warning => logger::warning_fmt!("{:?}", anyhow::Error::from(self)),
				LogType::Error => logger::error_fmt!("{:?}", anyhow::Error::from(self)),
			}
		} else if self.source.is_some() {
			match self.log_type {
				LogType::Debug => logger::debug_fmt!("{}", self.source.unwrap()),
				LogType::Warning => logger::warning_fmt!("{:?}", self.source.unwrap()),
				LogType::Error => logger::error_fmt!("{:?}", self.source.unwrap()),
			}
		} else {
			// Nothing to log...
		}
	}
}

impl Default for GovanErrorImpl {
	fn default() -> Self {
		GovanErrorImpl {
			log_type: LogType::Error,
			to_log: None,
			to_user: UserMsgType::None,
			source: None,
		}
	}
}

impl std::fmt::Display for GovanErrorImpl {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(err_msg) = self.to_log.as_ref() {
			write!(f, "{}", err_msg)?;
		}

		Ok(())
	}
}

macro_rules! fmt_or_str_or_none {
	() => {
		Option::<String>::None
	};
	(() or ) => {
		Option::<String>::None
	};
	(() or $entity:expr) => {
		Some($entity)
	};
	(($entity:tt) or ) => {
		Some(format!$entity)
	};
	(($__:tt) or $_:expr) => {
		compile_error!("Invalid combination for fmt_or_str_or_none")
	};
}

macro_rules! user_fmt_or_str_or_none {
	() => {
		$crate::util::error::UserMsgType::None
	};
	(() or ) => {
		$crate::util::error::UserMsgType::None
	};
	(() or $entity:expr) => {
		$crate::util::error::UserMsgType::Normal($entity.into())
	};
	(($entity:tt) or ) => {
		$crate::util::error::UserMsgType::Normal(format!$entity)
	};
	(($__:tt) or $_:expr) => {
		compile_error!("Invalid combination for fmt_or_str_or_none")
	};
}

macro_rules! create_error {
	($iden:ident,) => {
		$crate::util::error::GovanError::$iden(
			Option::<String>::None,
			$crate::util::error::UserMsgType::None,
		)
	};
  ($iden:ident, user $(fmt = $userfmt:tt)?$(= $user:expr)?) => {
    $crate::util::error::GovanError::$iden(
			Option::<String>::None,
			$crate::util::error::user_fmt_or_str_or_none!(($($userfmt)?) or $($user)?)
		)
  };
  ($iden:ident, log $(fmt = $logfmt:tt)?$( = $log:expr)? $(, user $(fmt = $userfmt:tt)?$(= $user:expr)?)?) => {
		$crate::util::error::GovanError::$iden(
			$crate::util::error::fmt_or_str_or_none!(($($logfmt)?) or $($log)?),
			$crate::util::error::user_fmt_or_str_or_none!($(($($userfmt)?) or $($user)?)?)
		)
	};
}

#[macro_export]
macro_rules! error {
  ($($tt:tt)*) => {
    $crate::util::error::create_error!(as_error, $($tt)*)
  };
}

#[macro_export]
macro_rules! error_map {
  ($($tt:tt)*) => {
    |e| $crate::util::error::create_error!(as_error, $($tt)*).with_source(e)
  };
}

#[macro_export]
macro_rules! error_lazy {
  ($($tt:tt)*) => {
    || $crate::util::error::create_error!(as_error, $($tt)*)
  };
}

#[macro_export]
macro_rules! warning {
  ($($tt:tt)*) => {
    $crate::util::error::create_error!(as_warning, $($tt)*)
  };
}

#[macro_export]
macro_rules! warning_map {
  ($($tt:tt)*) => {
    |e| $crate::util::error::create_error!(as_warning, $($tt)*).with_source(e)
  };
}

#[macro_export]
macro_rules! warning_lazy {
  ($($tt:tt)*) => {
    || $crate::util::error::create_error!(as_warning, $($tt)*)
  };
}

#[macro_export]
macro_rules! debug {
  ($($tt:tt)*) => {
    $crate::util::error::create_error!(as_debug, $($tt)*)
  };
}

#[macro_export]
macro_rules! debug_map {
  ($($tt:tt)*) => {
    |e| $crate::util::error::create_error!(as_debug, $($tt)*).with_source(e)
  };
}

#[macro_export]
macro_rules! debug_lazy {
  ($($tt:tt)*) => {
    || $crate::util::error::create_error!(as_debug, $($tt)*)
  };
}

pub(crate) use create_error;
pub(crate) use debug;
pub(crate) use debug_lazy;
pub(crate) use debug_map;
pub(crate) use error;
pub(crate) use error_lazy;
pub(crate) use error_map;
pub(crate) use fmt_or_str_or_none;
pub(crate) use user_fmt_or_str_or_none;
// pub(crate) use warning;
pub(crate) use warning_lazy;
// pub(crate) use warning_map;
