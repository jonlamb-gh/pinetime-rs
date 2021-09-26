use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use pinetime_graphics::embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::RgbColor,
    prelude::*,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use pinetime_graphics::{
    display::{self, PixelFormat, BACKGROUND_COLOR},
    font_styles::FontStyles,
    icons::{Icon, Icons},
};
use std::{thread, time::Duration};

const SIMULATOR_SCALE: u32 = 2;

fn main() -> Result<(), core::convert::Infallible> {
    let mut display =
        SimulatorDisplay::<PixelFormat>::with_default_color(display::SIZE, BACKGROUND_COLOR);
    let output_settings = OutputSettingsBuilder::new().scale(SIMULATOR_SCALE).build();
    let mut window = Window::new("PineTime", &output_settings);

    let font_styles = FontStyles::default();
    let icons = Icons::default();

    'running: loop {
        window.update(&display);

        clear_screen(&mut display)?;

        let text = "12:20";
        time(&font_styles, text).draw(&mut display)?;

        let text = "FRI 24 SEP 2021";
        date(&font_styles, text).draw(&mut display)?;

        battery_icon(&icons).draw(&mut display)?;
        charge_icon(&icons).draw(&mut display)?;

        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'running,
                SimulatorEvent::MouseButtonDown { point, .. } => {
                    println!("Down {:?}", point);
                }
                SimulatorEvent::MouseButtonUp { point, .. } => {
                    println!("Up {:?}", point);
                }
                _ => {}
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}

fn clear_screen<D>(target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = PixelFormat>,
{
    target.clear(BACKGROUND_COLOR)?;
    Ok(())
}

fn time<'a>(fs: &FontStyles, time_str: &'a str) -> Text<'a, MonoTextStyle<'a, PixelFormat>> {
    let font_style = fs.watchface_time_style;
    let text_style = TextStyleBuilder::new()
        .baseline(Baseline::Alphabetic)
        .alignment(Alignment::Center)
        .build();
    let pos_x = (display::WIDTH / 2) as i32;
    let pos_y = (display::HEIGHT / 2) as i32;
    Text::with_text_style(time_str, Point::new(pos_x, pos_y), font_style, text_style)
}

fn date<'a>(fs: &FontStyles, date_str: &'a str) -> Text<'a, MonoTextStyle<'a, PixelFormat>> {
    let font_style = fs.watchface_date_style;
    let text_style = TextStyleBuilder::new()
        .baseline(Baseline::Alphabetic)
        .alignment(Alignment::Center)
        .build();
    let pos_x = (display::WIDTH / 2) as i32;
    let pos_y = (display::HEIGHT / 2) as i32 + 50;
    Text::with_text_style(date_str, Point::new(pos_x, pos_y), font_style, text_style)
}

fn battery_icon<'a>(icons: &Icons) -> Text<'a, MonoTextStyle<'a, PixelFormat>> {
    let icon_style = MonoTextStyleBuilder::new()
        .font(icons.p20)
        .text_color(PixelFormat::WHITE)
        .build();
    let pos_x = display::WIDTH - 30;
    let pos_y = 20;
    Text::new(
        Icon::BatteryFull.as_text(),
        Point::new(pos_x as _, pos_y),
        icon_style,
    )
}

fn charge_icon<'a>(icons: &Icons) -> Text<'a, MonoTextStyle<'a, PixelFormat>> {
    let icon_style = MonoTextStyleBuilder::new()
        .font(icons.p20)
        .text_color(PixelFormat::RED)
        .build();
    let pos_x = display::WIDTH - 55;
    let pos_y = 22;
    Text::new(
        Icon::Plug.as_text(),
        Point::new(pos_x as _, pos_y),
        icon_style,
    )
}
