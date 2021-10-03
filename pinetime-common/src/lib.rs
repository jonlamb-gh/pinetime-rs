#![no_std]

pub use crate::animated_display::{AnimatedDisplay, RefreshDirection};
pub use crate::battery_controller::{BatteryControllerExt, MilliVolts};
pub use crate::system_time::SystemTimeExt;
pub use chrono;
pub use embedded_graphics;
pub use err_derive;

mod animated_display;
mod battery_controller;
pub mod display;
mod system_time;
