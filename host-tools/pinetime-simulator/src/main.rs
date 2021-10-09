use chrono::{DateTime, Local, NaiveDateTime};
use embedded_graphics_simulator::{
    sdl2::Keycode, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use pinetime_common::{
    display::{self, PixelFormat, BACKGROUND_COLOR},
    embedded_graphics::prelude::*,
    BatteryControllerExt, MilliVolts, SystemTimeExt,
};
use pinetime_graphics::{
    font_styles::FontStyles,
    icons::Icons,
    screens::{WatchFace, WatchFaceResources},
};
use std::{thread, time::Duration};

const SIMULATOR_SCALE: u32 = 2;

const FONT_STYLES: FontStyles = FontStyles::new();
const ICONS: Icons = Icons::new();

fn main() -> Result<(), core::convert::Infallible> {
    let mut display =
        SimulatorDisplay::<PixelFormat>::with_default_color(display::SIZE, BACKGROUND_COLOR);
    let output_settings = OutputSettingsBuilder::new().scale(SIMULATOR_SCALE).build();
    let mut window = Window::new("PineTime Simulator", &output_settings);

    let mut sim_clock = SimClock::default();
    let mut sim_battery = SimBattery::default();

    let mut screen = WatchFace::default();

    clear_screen(&mut display)?;

    'running: loop {
        window.update(&display);

        sim_clock.update();

        let res = WatchFaceResources {
            font_styles: &FONT_STYLES,
            icons: &ICONS,
            sys_time: &sim_clock,
            bat_ctl: &sim_battery,
        };

        screen.refresh(&mut display, &res).unwrap();

        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'running,
                SimulatorEvent::MouseButtonDown { point, .. } => {
                    println!("Down {:?}", point);
                }
                SimulatorEvent::MouseButtonUp { point, .. } => {
                    println!("Up {:?}", point);
                }
                SimulatorEvent::KeyDown {
                    keycode,
                    keymod: _,
                    repeat,
                } => {
                    if !repeat {
                        match keycode {
                            Keycode::B => {
                                sim_battery.percent_remaining += 10;
                                if sim_battery.percent_remaining > 100 {
                                    sim_battery.percent_remaining = 0;
                                }
                                println!("Battery {} %", sim_battery.percent_remaining);
                            }
                            Keycode::C => {
                                sim_battery.charging = !sim_battery.charging;
                            }
                            _ => (),
                        }
                    }
                }
                _ => {}
            }
        }

        thread::sleep(Duration::from_millis(20));
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

pub struct SimClock {
    pub dt: NaiveDateTime,
}

impl Default for SimClock {
    fn default() -> Self {
        let mut sc = SimClock {
            dt: NaiveDateTime::from_timestamp(0, 0),
        };
        sc.update();
        sc
    }
}

impl SimClock {
    pub fn update(&mut self) {
        let now: DateTime<Local> = Local::now();
        self.dt = NaiveDateTime::from_timestamp(now.timestamp(), now.timestamp_subsec_nanos());
    }
}

impl SystemTimeExt for SimClock {
    fn date_time(&self) -> &NaiveDateTime {
        &self.dt
    }
}

pub struct SimBattery {
    pub charging: bool,
    pub voltage: MilliVolts,
    pub percent_remaining: u8,
}

impl Default for SimBattery {
    fn default() -> Self {
        SimBattery {
            charging: false,
            voltage: MilliVolts(4180), // max, 4.21v
            percent_remaining: 100,
        }
    }
}

impl BatteryControllerExt for SimBattery {
    fn is_charging(&self) -> bool {
        self.charging
    }

    fn voltage(&self) -> MilliVolts {
        self.voltage
    }

    fn percent_remaining(&self) -> u8 {
        self.percent_remaining
    }
}
