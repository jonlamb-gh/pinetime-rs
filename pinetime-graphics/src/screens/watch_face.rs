use crate::{
    display::{self, PixelFormat, BACKGROUND_COLOR},
    font_styles::FontStyles,
    icons::{Icon, Icons},
};
use core::fmt::{self, Write};
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
    Drawable,
};
use heapless::String;
use pinetime_common::{
    chrono::{NaiveDateTime, Timelike},
    SystemTimeExt,
};

// common error type in this crate or common crate
// let main/bin unwrap

pub struct WatchFaceResources<'a, T: SystemTimeExt> {
    pub sys_time: &'a T,
    pub font_styles: &'a FontStyles,
    pub icons: &'a Icons,
}

pub struct WatchFace {
    dt: NaiveDateTime,
    text: String<32>,
}

impl Default for WatchFace {
    fn default() -> Self {
        Self::new()
    }
}

impl WatchFace {
    pub fn new() -> Self {
        WatchFace {
            dt: NaiveDateTime::from_timestamp(0, 0),
            text: String::new(),
        }
    }

    pub fn refresh<'a, D, T>(&mut self, display: &mut D, res: &WatchFaceResources<'a, T>)
    where
        D: DrawTarget<Color = PixelFormat>,
        <D as DrawTarget>::Error: fmt::Debug,
        T: SystemTimeExt,
    {
        // TODO - check if visible components changed first...

        let dt = res.sys_time.date_time();

        self.dt = *dt;

        self.draw_time(display, res);
        self.draw_date(display, res);
    }

    fn draw_time<'a, D, T>(&mut self, display: &mut D, res: &WatchFaceResources<'a, T>)
    where
        D: DrawTarget<Color = PixelFormat>,
        <D as DrawTarget>::Error: fmt::Debug,
        T: SystemTimeExt,
    {
        let time = self.dt.time();
        self.text.clear();
        write!(&mut self.text, "{:02}:{:02}", time.minute(), time.second()).unwrap();
        //write!(display_string, "{}:{}", t.hour(), t.minute()).unwrap();

        let mut font_style = res.font_styles.watchface_time_style;
        font_style.background_color = BACKGROUND_COLOR.into();
        let text_style = TextStyleBuilder::new()
            .baseline(Baseline::Alphabetic)
            .alignment(Alignment::Center)
            .build();
        let pos_x = (display::WIDTH / 2) as i32;
        let pos_y = (display::HEIGHT / 2) as i32;
        Text::with_text_style(&self.text, Point::new(pos_x, pos_y), font_style, text_style)
            .draw(display)
            .unwrap();
    }

    fn draw_date<'a, D, T>(&mut self, display: &mut D, res: &WatchFaceResources<'a, T>)
    where
        D: DrawTarget<Color = PixelFormat>,
        <D as DrawTarget>::Error: fmt::Debug,
        T: SystemTimeExt,
    {
        // TODO
        self.text.clear();
        write!(&mut self.text, "FRI 24 SEP 2021").unwrap();

        let mut font_style = res.font_styles.watchface_date_style;
        font_style.background_color = BACKGROUND_COLOR.into();
        let text_style = TextStyleBuilder::new()
            .baseline(Baseline::Alphabetic)
            .alignment(Alignment::Center)
            .build();
        let pos_x = (display::WIDTH / 2) as i32;
        let pos_y = (display::HEIGHT / 2) as i32 + 50;
        Text::with_text_style(&self.text, Point::new(pos_x, pos_y), font_style, text_style)
            .draw(display)
            .unwrap();
    }
}
