// TODO use https://docs.rs/embedded-graphics/0.7.1/embedded_graphics/draw_target/trait.DrawTargetExt.html
// to clip while scrolling

use crate::hal::prelude::{OutputPin, _embedded_hal_blocking_delay_DelayUs as DelayUs};
use display_interface::WriteOnlyDataCommand;
use pinetime_common::embedded_graphics::{
    draw_target::DrawTarget, pixelcolor::Rgb565, prelude::*, primitives::Rectangle,
};
use pinetime_common::{display, AnimatedDisplay, RefreshDirection};
use st7789::{Error, Orientation, ST7789};

pub const SCROLL_DELTA: u16 = 16;

pub struct AnimatedSt7789<DI, RST>
where
    DI: WriteOnlyDataCommand,
    RST: OutputPin,
{
    in_progress_animation: Option<RefreshDirection>,
    scroll_offset: u16,
    display: ST7789<DI, RST>,
}

impl<DI, RST, PinE> AnimatedSt7789<DI, RST>
where
    DI: WriteOnlyDataCommand,
    RST: OutputPin<Error = PinE>,
{
    pub fn new(di: DI, rst: RST, size_x: u16, size_y: u16) -> Self {
        AnimatedSt7789 {
            in_progress_animation: None,
            scroll_offset: 0,
            display: ST7789::new(di, rst, size_x, size_y),
        }
    }

    pub fn init(&mut self, delay_source: &mut impl DelayUs<u32>) -> Result<(), Error<PinE>> {
        self.display.init(delay_source)?;
        self.display.set_orientation(Orientation::Portrait)?;
        Ok(())
    }
}

impl<DI, OUT, PinE> DrawTarget for AnimatedSt7789<DI, OUT>
where
    DI: WriteOnlyDataCommand,
    OUT: OutputPin<Error = PinE>,
{
    type Error = Error<PinE>;
    type Color = Rgb565;

    fn draw_iter<T>(&mut self, item: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = Pixel<Rgb565>>,
    {
        self.display.draw_iter(item)
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.display.fill_contiguous(area, colors)
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        self.display.fill_solid(area, color)
    }

    fn clear(&mut self, color: Rgb565) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        self.display.clear(color)
    }
}

impl<DI, OUT, PinE> OriginDimensions for AnimatedSt7789<DI, OUT>
where
    DI: WriteOnlyDataCommand,
    OUT: OutputPin<Error = PinE>,
{
    fn size(&self) -> Size {
        self.display.size()
    }
}

impl<DI, OUT, PinE> AnimatedDisplay for AnimatedSt7789<DI, OUT>
where
    DI: WriteOnlyDataCommand,
    OUT: OutputPin<Error = PinE>,
{
    type Error = Error<PinE>;

    fn set_refresh_direction(&mut self, refresh_dir: RefreshDirection) {
        if self.in_progress_animation.is_none() {
            self.in_progress_animation = refresh_dir.into();
            self.scroll_offset = match refresh_dir {
                RefreshDirection::Up => 0,
                RefreshDirection::Down => display::VERT_LINES,
            };
        }
    }

    // TODO - instead of Clipped+force-redraws, do fill_solid in here to fill background as
    // scrolling progresses
    // rect size is (width, SCROLL_DELTA), offset moves with scroll_offset
    fn update_animations(&mut self) -> Result<(), Error<PinE>> {
        let is_done = match self.in_progress_animation {
            Some(RefreshDirection::Up) => {
                self.display.set_scroll_offset(self.scroll_offset)?;
                self.scroll_offset += SCROLL_DELTA;
                self.scroll_offset = self.scroll_offset.clamp(0, display::VERT_LINES);
                self.scroll_offset == display::VERT_LINES
            }
            Some(RefreshDirection::Down) => {
                self.display.set_scroll_offset(self.scroll_offset)?;
                self.scroll_offset = self.scroll_offset.wrapping_sub(SCROLL_DELTA);
                self.scroll_offset = self.scroll_offset.clamp(0, display::VERT_LINES);
                self.scroll_offset == display::VERT_LINES
            }
            _ => true,
        };
        if is_done {
            self.in_progress_animation = None;
        }
        Ok(())
    }
}
