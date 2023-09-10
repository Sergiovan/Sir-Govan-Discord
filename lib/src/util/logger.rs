#![allow(dead_code)]

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use colored::Colorize;
use lazy_static::lazy_static;

use std::sync::Mutex;

lazy_static! {
	static ref DAY: Mutex<DateTime<Utc>> =
		Mutex::new(Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap());
	static ref DEBUG_TEXT: String = format!("[{:7}]", "DEBUG".green());
	static ref INFO_TEXT: String = format!("[{:7}]", "INFO".cyan());
	static ref WARNING_TEXT: String = format!("[{:7}]", "WARNING".yellow());
	static ref ERROR_TEXT: String = format!("[{:7}]", "ERROR".red());
}

#[macro_export]
macro_rules! debug_fmt {
  ($($tt:tt)*) => {
    $crate::util::logger::debug(&format!($($tt)*))
  };
}

#[macro_export]
macro_rules! info_fmt {
  ($($tt:tt)*) => {
    $crate::util::logger::info(&format!($($tt)*))
  };
}

#[macro_export]
macro_rules! warning_fmt {
  ($($tt:tt)*) => {
    $crate::util::logger::warning(&format!($($tt)*))
  };
}

#[macro_export]
macro_rules! error_fmt {
  ($($tt:tt)*) => {
    $crate::util::logger::error(&format!($($tt)*))
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
	let timestamp = chrono::offset::Utc::now().timestamp_micros().unsigned_abs();
	let error_code = timestamp.rotate_right(17) ^ timestamp;
	let error_code = error_code ^ error_code >> 12;
	let error_code = error_code ^ error_code << 21;
	let error_code = error_code.wrapping_mul(57436991);
	let error_code = format!("{:016X}", error_code).red();

	let error_text = format!("{} -> {}", error_code, text);

	eprintln!("{}", error_text);

	print_message(Utc::now(), &ERROR_TEXT, &error_text);
}

fn print_message(time: DateTime<Utc>, level: &str, text: &str) {
	let lock = DAY.lock().unwrap();
	let day_passed = {
		let day = &*lock;
		day.day() != time.day() || day.month() != time.month() || day.year() != time.year()
	};

	if day_passed {
		let mut day = lock;
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

	let time_str = time.format("%H:%M:%S%.3f");
	let header = format!("[{}]{} ", time_str, level);
	let rest = format!("              {} ", level);
	text.split('\n')
		.enumerate()
		.for_each(|(i, s)| println!("{}{}", if i == 0 { &header } else { &rest }, s));
}
