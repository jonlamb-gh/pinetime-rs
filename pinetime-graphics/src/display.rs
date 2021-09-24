use embedded_graphics::{geometry::Size, pixelcolor::Rgb565, prelude::RgbColor};

pub const WIDTH: u16 = 240;
pub const HEIGHT: u16 = 240;
pub const SIZE: Size = Size::new(WIDTH as u32, HEIGHT as u32);

pub type PixelFormat = Rgb565;
pub const BACKGROUND_COLOR: PixelFormat = PixelFormat::BLACK;
