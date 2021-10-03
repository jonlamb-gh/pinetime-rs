use core::sync::atomic::{AtomicBool, Ordering::SeqCst};
use embedded_graphics::{geometry::Size, pixelcolor::Rgb565, prelude::RgbColor};

pub const WIDTH: u16 = 240;
pub const HEIGHT: u16 = 240;
pub const VERT_LINES: u16 = 320;
pub const SIZE: Size = Size::new(WIDTH as u32, HEIGHT as u32);

pub type PixelFormat = Rgb565;
pub const BACKGROUND_COLOR: PixelFormat = PixelFormat::BLACK;

#[derive(Debug)]
#[repr(transparent)]
pub struct AtomicDisplayAwakeState(AtomicBool);

impl AtomicDisplayAwakeState {
    pub const fn new(initial_state: bool) -> Self {
        AtomicDisplayAwakeState(AtomicBool::new(initial_state))
    }

    pub fn awaken(&self) {
        self.0.store(true, SeqCst);
    }

    pub fn is_awake(&self) -> bool {
        self.0.load(SeqCst)
    }

    pub fn get_and_clear(&self) -> bool {
        self.0.swap(false, SeqCst)
    }
}
