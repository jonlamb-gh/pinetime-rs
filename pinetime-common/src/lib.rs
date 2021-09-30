#![no_std]

pub use crate::battery_controller::{BatteryControllerExt, MilliVolts};
pub use chrono;
use chrono::NaiveDateTime;

mod battery_controller;

pub trait SystemTimeExt {
    fn date_time(&self) -> &NaiveDateTime;
}
