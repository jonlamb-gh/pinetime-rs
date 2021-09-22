use crate::hal::gpio::{p0, Output, PushPull};
use embedded_graphics::pixelcolor::Rgb565;

pub type PixelFormat = Rgb565;

pub const WIDTH: u16 = 240;
pub const HEIGHT: u16 = 240;

pub type LcdCsPin = p0::P0_25<Output<PushPull>>;
pub type LcdDcPin = p0::P0_18<Output<PushPull>>;
pub type LcdResetPin = p0::P0_26<Output<PushPull>>;
