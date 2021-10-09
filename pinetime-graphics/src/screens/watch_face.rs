use crate::{
    font_styles::FontStyles,
    icons::{Icon, Icons},
};
use bitflags::bitflags;
use core::fmt::Write;
use heapless::String;
use pinetime_common::embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::MonoTextStyleBuilder,
    prelude::*,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
    Drawable,
};
use pinetime_common::{
    chrono::{Datelike, NaiveDateTime, Timelike},
    display::{self, PixelFormat, BACKGROUND_COLOR},
    err_derive, BatteryControllerExt, SystemTimeExt,
};

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

#[derive(Debug, err_derive::Error)]
pub enum Error {
    #[error(display = "Formatting error")]
    Formatting(#[error(source)] core::fmt::Error),
}

pub struct WatchFaceResources<'a, T: SystemTimeExt, B: BatteryControllerExt> {
    pub sys_time: &'a T,
    pub bat_ctl: &'a B,
}

pub struct WatchFace {
    redraw: Redraw,
    dt: NaiveDateTime,
    is_charging: bool,
    battery_icon: Icon,
    time_text: String<6>,
    date_text: String<18>,
    font_styles: &'static FontStyles,
    icons: &'static Icons,
}

bitflags! {
    struct Redraw: u8 {
        const ALL = 0xFF;
        const TIME = 1 << 0;
        const DATE = 1 << 1;
        const BATTERY = 1 << 2;
        const CHARGE_PLUG = 1 << 3;
        const FORCE_UPDATE = 1 << 7;
    }
}

impl Redraw {
    fn clear(&mut self) {
        self.bits = 0;
    }

    fn set_all(&mut self) {
        self.bits = Self::ALL.bits;
    }
}

impl WatchFace {
    pub fn new(font_styles: &'static FontStyles, icons: &'static Icons) -> Self {
        WatchFace {
            redraw: Redraw::ALL,
            dt: NaiveDateTime::from_timestamp(0, 0),
            is_charging: false,
            battery_icon: Icon::BatteryFull,
            time_text: String::new(),
            date_text: String::new(),
            font_styles,
            icons,
        }
    }

    pub fn force_redraw(&mut self) {
        self.redraw.set_all();
    }

    pub fn clear_redraw(&mut self) {
        self.redraw.clear();
    }

    pub fn update<'a, T, B>(&mut self, res: &WatchFaceResources<'a, T, B>) -> Result<(), Error>
    where
        T: SystemTimeExt,
        B: BatteryControllerExt,
    {
        let dt = res.sys_time.date_time();
        let percent_remaining = res.bat_ctl.percent_remaining();
        let is_charging = res.bat_ctl.is_charging();

        self.update_date_time(dt)?;
        self.update_battery_indicator(percent_remaining);
        self.update_battery_charge_plug(is_charging);

        Ok(())
    }

    fn update_date_time(&mut self, dt: &NaiveDateTime) -> Result<(), Error> {
        let mut changed = false;

        let prev_date = self.dt.date();
        let date = dt.date();
        if self.redraw.contains(Redraw::FORCE_UPDATE) || prev_date != date {
            self.date_text.clear();
            write!(
                &mut self.date_text,
                "{} {:02} {} {}",
                date.weekday(),
                date.day(),
                MONTHS[date.month0().clamp(0, 11) as usize],
                date.year()
            )?;
            self.redraw |= Redraw::DATE;
            changed = true;
        }

        let prev_time = self.dt.time();
        let time = dt.time();
        if self.redraw.contains(Redraw::FORCE_UPDATE)
            || prev_time.hour12() != time.hour12()
            || prev_time.minute() != time.minute()
        {
            self.time_text.clear();
            write!(
                &mut self.time_text,
                "{:02}:{:02}",
                time.hour12().1,
                time.minute()
            )?;
            self.redraw |= Redraw::TIME;
            changed = true;
        }

        if changed {
            self.dt = *dt;
        }

        Ok(())
    }

    fn update_battery_indicator(&mut self, percent_remaining: u8) {
        let icon = Icon::battery_icon_from_percent_remaining(percent_remaining);
        if icon != self.battery_icon {
            self.redraw |= Redraw::BATTERY;
            self.battery_icon = icon;
        }
    }

    fn update_battery_charge_plug(&mut self, is_charging: bool) {
        if is_charging != self.is_charging {
            self.redraw |= Redraw::CHARGE_PLUG;
            self.is_charging = is_charging;
        }
    }

    fn draw_time<D>(&self, display: &mut D) -> Result<(), <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = PixelFormat>,
    {
        if self.redraw.contains(Redraw::TIME) {
            let mut font_style = self.font_styles.watchface_time.style();
            font_style.background_color = BACKGROUND_COLOR.into();
            let text_style = TextStyleBuilder::new()
                .baseline(Baseline::Alphabetic)
                .alignment(Alignment::Center)
                .build();
            let pos_x = (display::WIDTH / 2) as i32;
            let pos_y = (display::HEIGHT / 2) as i32;
            Text::with_text_style(
                &self.time_text,
                Point::new(pos_x, pos_y),
                font_style,
                text_style,
            )
            .draw(display)?;
        }

        Ok(())
    }

    fn draw_date<D>(&self, display: &mut D) -> Result<(), <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = PixelFormat>,
    {
        if self.redraw.contains(Redraw::DATE) {
            let mut font_style = self.font_styles.watchface_date.style();
            font_style.background_color = BACKGROUND_COLOR.into();
            let text_style = TextStyleBuilder::new()
                .baseline(Baseline::Alphabetic)
                .alignment(Alignment::Center)
                .build();
            let pos_x = (display::WIDTH / 2) as i32;
            let pos_y = (display::HEIGHT / 2) as i32 + 50;
            Text::with_text_style(
                &self.date_text,
                Point::new(pos_x, pos_y),
                font_style,
                text_style,
            )
            .draw(display)?;
        }
        Ok(())
    }

    fn draw_battery_indicator<D>(&self, display: &mut D) -> Result<(), <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = PixelFormat>,
    {
        if self.redraw.contains(Redraw::BATTERY) {
            let color = if self.battery_icon == Icon::BatteryEmpty {
                display::PixelFormat::RED
            } else {
                display::PixelFormat::WHITE
            };

            let icon_style = MonoTextStyleBuilder::new()
                .font(self.icons.p20)
                .text_color(color)
                .background_color(BACKGROUND_COLOR)
                .build();
            let pos_x = display::WIDTH - 30;
            let pos_y = 20;
            Text::new(
                self.battery_icon.as_text(),
                Point::new(pos_x as _, pos_y),
                icon_style,
            )
            .draw(display)?;
        }

        Ok(())
    }

    fn draw_battery_charge_plug<D>(&self, display: &mut D) -> Result<(), <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = PixelFormat>,
    {
        if self.redraw.contains(Redraw::CHARGE_PLUG) {
            let color = if self.is_charging {
                display::PixelFormat::RED
            } else {
                display::BACKGROUND_COLOR
            };

            let icon_style = MonoTextStyleBuilder::new()
                .font(self.icons.p20)
                .text_color(color)
                .build();
            let pos_x = display::WIDTH - 55;
            let pos_y = 22;
            Text::new(
                Icon::Plug.as_text(),
                Point::new(pos_x as _, pos_y),
                icon_style,
            )
            .draw(display)?;
        }

        Ok(())
    }
}

impl Drawable for WatchFace {
    type Color = PixelFormat;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = PixelFormat>,
    {
        self.draw_time(target)?;
        self.draw_date(target)?;
        self.draw_battery_indicator(target)?;
        self.draw_battery_charge_plug(target)?;
        Ok(())
    }
}
