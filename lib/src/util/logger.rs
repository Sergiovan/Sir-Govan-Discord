#![allow(dead_code)]

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use colored::Colorize;
use lazy_static::lazy_static;

use std::sync::RwLock;

lazy_static! {
	static ref DAY: RwLock<DateTime<Utc>> =
		RwLock::new(Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap());
	static ref DEBUG_TEXT: String = format!("[{:7}]", "DEBUG".green());
	static ref INFO_TEXT: String = format!("[{:7}]", "INFO".cyan());
	static ref WARNING_TEXT: String = format!("[{:7}]", "WARNING".yellow());
	static ref ERROR_TEXT: String = format!("[{:7}]", "ERROR".red());
}

#[macro_export]
macro_rules! debug_fmt {
  ($($tt:tt)*) => {
    $crate::util::logger::debug(&format!($($tt)*));
  };
}

#[macro_export]
macro_rules! info_fmt {
  ($($tt:tt)*) => {
    $crate::util::logger::info(&format!($($tt)*));
  };
}

#[macro_export]
macro_rules! warning_fmt {
  ($($tt:tt)*) => {
    $crate::util::logger::warning(&format!($($tt)*));
  };
}

#[macro_export]
macro_rules! error_fmt {
  ($($tt:tt)*) => {
    $crate::util::logger::error(&format!($($tt)*));
  };
}

#[allow(unused_imports)]
pub(crate) use debug_fmt;
#[allow(unused_imports)]
pub(crate) use error_fmt;
#[allow(unused_imports)]
pub(crate) use info_fmt;
#[allow(unused_imports)]
pub(crate) use warning_fmt;

pub fn debug(text: &str) {
	print_message(Utc::now(), &DEBUG_TEXT, text);
}

pub fn info(text: &str) {
	print_message(Utc::now(), &INFO_TEXT, text);
}

pub fn warning(text: &str) {
	print_message(Utc::now(), &WARNING_TEXT, text);
}

pub fn error(text: &str) {
	print_message(Utc::now(), &ERROR_TEXT, text);
}

fn print_message(time: DateTime<Utc>, level: &str, text: &str) {
	{
		let day_passed = {
			let day = DAY.read().expect("Could not lock day");
			day.day() != time.day() || day.month() != time.month() || day.year() != time.year()
		};

		if day_passed {
			let mut day = DAY.write().expect("Could not lock day");
			if day.year() == 2000 {
				println!("~~~~~~~ {} ~~~~~~~", time.format("%Y-%m-%d"));
				*day = time;
			} else {
				while *day <= time {
					*day += Duration::days(1);
					println!("~~~~~~~ {} ~~~~~~~", day.format("%Y-%m-%d"));
				}
			}
		}
	}

	let time_str = time.format("%H:%M:%S%.3f");
	println!("[{}]{} {}", time_str, level, text);
}
