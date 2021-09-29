#![no_std]

pub use chrono;

use chrono::NaiveDateTime;

pub trait SystemTimeExt {
    fn date_time(&self) -> &NaiveDateTime;
}
