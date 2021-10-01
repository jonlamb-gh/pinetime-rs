use crate::{
    display::{self, PixelFormat, BACKGROUND_COLOR},
    font_styles::FontStyles,
    icons::{Icon, Icons},
};
use core::fmt::{self, Write};
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::MonoTextStyleBuilder,
    prelude::*,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
    Drawable,
};
use heapless::String;
use pinetime_common::{
    chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike},
    BatteryControllerExt, SystemTimeExt,
};

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

#[derive(Debug, err_derive::Error)]
pub enum Error {
    #[error(display = "DrawTarget error")]
    DrawTarget,

    #[error(display = "Formatting error")]
    Formatting(#[error(source)] core::fmt::Error),
}

pub struct WatchFaceResources<'a, T: SystemTimeExt, B: BatteryControllerExt> {
    pub font_styles: &'a FontStyles,
    pub icons: &'a Icons,
    pub sys_time: &'a T,
    pub bat_ctl: &'a B,
}

pub struct WatchFace {
    dt: NaiveDateTime,
    is_charging: bool,
    battery_icon: Icon,
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
            is_charging: false,
            battery_icon: Icon::BatteryFull,
            text: String::new(),
        }
    }

    pub fn refresh<'a, D, T, B>(
        &mut self,
        display: &mut D,
        res: &WatchFaceResources<'a, T, B>,
    ) -> Result<(), Error>
    where
        D: DrawTarget<Color = PixelFormat>,
        <D as DrawTarget>::Error: fmt::Debug,
        T: SystemTimeExt,
        B: BatteryControllerExt,
    {
        // Force a full redraw on startup
        let force = self.text.is_empty();

        let dt = res.sys_time.date_time();
        let date = dt.date();
        let time = dt.time();

        self.draw_time(res, force, &time, display)?;
        self.draw_date(res, force, &date, display)?;
        self.draw_battery_indicator(res, force, display)?;
        self.draw_battery_charge_plug(res, force, display)?;

        self.dt = *dt;

        Ok(())
    }

    fn draw_time<'a, D, T, B>(
        &mut self,
        res: &WatchFaceResources<'a, T, B>,
        force: bool,
        time: &NaiveTime,
        display: &mut D,
    ) -> Result<(), Error>
    where
        D: DrawTarget<Color = PixelFormat>,
        <D as DrawTarget>::Error: fmt::Debug,
        T: SystemTimeExt,
        B: BatteryControllerExt,
    {
        let last_time = self.dt.time();
        if force || last_time.hour12() != time.hour12() || last_time.minute() != time.minute() {
            self.text.clear();
            write!(
                &mut self.text,
                "{:02}:{:02}",
                time.hour12().1,
                time.minute()
            )?;

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
                .map_err(|_| Error::DrawTarget)?;
        }

        Ok(())
    }

    fn draw_date<'a, D, T, B>(
        &mut self,
        res: &WatchFaceResources<'a, T, B>,
        force: bool,
        date: &NaiveDate,
        display: &mut D,
    ) -> Result<(), Error>
    where
        D: DrawTarget<Color = PixelFormat>,
        <D as DrawTarget>::Error: fmt::Debug,
        T: SystemTimeExt,
        B: BatteryControllerExt,
    {
        let last_date = self.dt.date();
        if force || last_date != *date {
            self.text.clear();
            write!(
                &mut self.text,
                "{} {:02} {} {}",
                date.weekday(),
                date.day(),
                MONTHS[date.month0().clamp(0, 11) as usize],
                date.year()
            )?;

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
                .map_err(|_| Error::DrawTarget)?;
        }
        Ok(())
    }

    fn draw_battery_indicator<'a, D, T, B>(
        &mut self,
        res: &WatchFaceResources<'a, T, B>,
        force: bool,
        display: &mut D,
    ) -> Result<(), Error>
    where
        D: DrawTarget<Color = PixelFormat>,
        <D as DrawTarget>::Error: fmt::Debug,
        T: SystemTimeExt,
        B: BatteryControllerExt,
    {
        let icon = Icon::battery_icon_from_percent_remaining(res.bat_ctl.percent_remaining());
        if force || icon != self.battery_icon {
            self.battery_icon = icon;

            let color = if icon == Icon::BatteryEmpty {
                display::PixelFormat::RED
            } else {
                display::PixelFormat::WHITE
            };

            let icon_style = MonoTextStyleBuilder::new()
                .font(res.icons.p20)
                .text_color(color)
                .background_color(BACKGROUND_COLOR)
                .build();
            let pos_x = display::WIDTH - 30;
            let pos_y = 20;
            Text::new(icon.as_text(), Point::new(pos_x as _, pos_y), icon_style)
                .draw(display)
                .map_err(|_| Error::DrawTarget)?;
        }

        Ok(())
    }

    fn draw_battery_charge_plug<'a, D, T, B>(
        &mut self,
        res: &WatchFaceResources<'a, T, B>,
        force: bool,
        display: &mut D,
    ) -> Result<(), Error>
    where
        D: DrawTarget<Color = PixelFormat>,
        <D as DrawTarget>::Error: fmt::Debug,
        T: SystemTimeExt,
        B: BatteryControllerExt,
    {
        let is_charging = res.bat_ctl.is_charging();
        if force || is_charging != self.is_charging {
            self.is_charging = is_charging;

            let color = if is_charging {
                display::PixelFormat::RED
            } else {
                display::BACKGROUND_COLOR
            };

            let icon_style = MonoTextStyleBuilder::new()
                .font(res.icons.p20)
                .text_color(color)
                .build();
            let pos_x = display::WIDTH - 55;
            let pos_y = 22;
            Text::new(
                Icon::Plug.as_text(),
                Point::new(pos_x as _, pos_y),
                icon_style,
            )
            .draw(display)
            .map_err(|_| Error::DrawTarget)?;
        }

        Ok(())
    }
}
