#![allow(dead_code)]

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use colored::Colorize;
use once_cell::sync::Lazy;

use std::{cell::Cell, sync::Mutex};

static DAY: Lazy<Mutex<Cell<DateTime<Utc>>>> = Lazy::new(|| {
	Mutex::new(Cell::new(
		Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(),
	))
});

pub fn debug(text: &str) {
	static DEBUG_TEXT: Lazy<String> = Lazy::new(|| format!("[{:7}]", "DEBUG".green()));

	print_message(Utc::now(), &DEBUG_TEXT, text);
}

pub fn info(text: &str) {
	static INFO_TEXT: Lazy<String> = Lazy::new(|| format!("[{:7}]", "INFO".cyan()));

	print_message(Utc::now(), &INFO_TEXT, text);
}

pub fn warning(text: &str) {
	static WARNING_TEXT: Lazy<String> = Lazy::new(|| format!("[{:7}]", "WARNING".yellow()));

	print_message(Utc::now(), &WARNING_TEXT, text);
}

pub fn error(text: &str) {
	static WARNING_TEXT: Lazy<String> = Lazy::new(|| format!("[{:7}]", "ERROR".red()));

	print_message(Utc::now(), &WARNING_TEXT, text);
}

fn print_message(time: DateTime<Utc>, level: &str, text: &str) {
	{
		let mut day_lock = DAY.lock().expect("Could not lock day");
		let day = day_lock.get_mut();
		if day.day() != time.day() || day.month() != time.month() || day.year() != time.year() {
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
