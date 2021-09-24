use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use pinetime_graphics::embedded_graphics::{
    mono_font::MonoTextStyle,
    prelude::*,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use pinetime_graphics::{
    display::{self, PixelFormat, BACKGROUND_COLOR},
    font_styles::FontStyles,
};
use std::{thread, time::Duration};

const SIMULATOR_SCALE: u32 = 2;

fn main() -> Result<(), core::convert::Infallible> {
    let mut display =
        SimulatorDisplay::<PixelFormat>::with_default_color(display::SIZE, BACKGROUND_COLOR);
    let output_settings = OutputSettingsBuilder::new().scale(SIMULATOR_SCALE).build();
    let mut window = Window::new("PineTime", &output_settings);

    let font_styles = FontStyles::default();

    'running: loop {
        window.update(&display);

        clear_screen(&mut display)?;

        let time = "12:20";
        draw_time(&font_styles, time).draw(&mut display)?;

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

fn draw_time<'a>(fs: &FontStyles, time_str: &'a str) -> Text<'a, MonoTextStyle<'a, PixelFormat>> {
    let font_style = fs.watchface_time_style;
    let text_style = TextStyleBuilder::new()
        .baseline(Baseline::Alphabetic)
        .alignment(Alignment::Center)
        .build();
    let pos_x = (display::WIDTH / 2) as i32;
    let pos_y = (display::HEIGHT / 2) as i32;
    Text::with_text_style(time_str, Point::new(pos_x, pos_y), font_style, text_style)
}
