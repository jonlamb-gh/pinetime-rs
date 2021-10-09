#![no_main]
#![no_std]

use nrf52832_hal as hal;
use panic_rtt_target as _;

mod rtc_monotonic;
mod system_time;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[rtic::app(device = crate::hal::pac, peripherals = true, dispatchers = [SWI0_EGU0, SWI1_EGU1, SWI2_EGU2, SWI3_EGU3, SWI4_EGU4])]
mod app {
    use crate::{built_info, hal, rtc_monotonic, system_time};
    use hal::{
        clocks::Clocks,
        gpio::{self, Level},
        gpiote::Gpiote,
        pac, ppi,
        prelude::*,
        spim::{self, Spim},
        timer::Timer,
        twim::{self, Frequency, Twim},
    };
    use pinetime_common::{
        display, embedded_graphics::prelude::*, AnimatedDisplay, AtomicDisplayAwakeState,
        RefreshDirection,
    };
    use pinetime_drivers::{
        animated_st7789::AnimatedSt7789,
        backlight::{Backlight, Brightness},
        battery_controller::BatteryController,
        button::Button,
        cst816s::{self, Cst816s, Gesture},
        display_interface_spi::SPIInterface,
        lcd::{LcdCsPin, LcdDcPin, LcdResetPin},
        motor_controller::MotorController,
        watchdog::Watchdog,
    };
    use pinetime_graphics::{
        font_styles::FontStyles,
        icons::Icons,
        screens::{WatchFace, WatchFaceResources},
    };
    use rtc_monotonic::{Rtc1Monotonic, RtcMonotonic};
    use rtic::time::duration::{Milliseconds, Seconds};
    use rtt_target::{rprintln, rtt_init_print};
    use system_time::SystemTime;

    const SCREEN_REFRESH_INTERVAL: Milliseconds = Milliseconds(20_u32);
    const DISPLAY_TIMEOUT: Seconds = Seconds(5_u32);
    const DISPLAY_TIMEOUT_POLL_INTERVAL: Seconds = Seconds(1_u32);
    const DISPLAY_TIMEOUT_TIMER_TICKS: u32 =
        Timer::<pac::TIMER0>::TICKS_PER_SECOND * DISPLAY_TIMEOUT.0;

    #[monotonic(binds = RTC1, default = true)]
    type RtcMono = Rtc1Monotonic;

    #[shared]
    struct Shared {
        display_state: AtomicDisplayAwakeState,

        #[lock_free]
        display_sleep_timer: Timer<pac::TIMER0>,

        #[lock_free]
        button: Button,

        #[lock_free]
        system_time: SystemTime<pac::RTC1, pac::TIMER1>,

        #[lock_free]
        backlight: Backlight,

        // Move to local, take a DisplayEvent arg, other tasks can send events to it
        // DisplayEvent::Refresh
        // DisplayEvent::ChargeInd(bool) or whatev
        // ...
        #[lock_free]
        display: AnimatedSt7789<SPIInterface<Spim<pac::SPIM0>, LcdDcPin, LcdCsPin>, LcdResetPin>,

        #[lock_free]
        battery_controller: BatteryController,

        #[lock_free]
        motor_controller: MotorController,
    }

    #[local]
    struct Local<'a> {
        gpiote: Gpiote,
        touch_controller: Cst816s<pac::TWIM1>,
        watchdog: Watchdog,
        watch_face: WatchFace,
    }

    #[init(local = [font_styles: FontStyles = FontStyles::new(), icons: Icons = Icons::new()])]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        rprintln!("Initializing");
        rprintln!("Version {}", built_info::PKG_VERSION);

        let hal::pac::Peripherals {
            CLOCK,
            P0,
            SPIM0,
            TIMER0,
            TIMER1,
            RTC1,
            PPI,
            GPIOTE,
            TWIM1,
            RADIO,
            SAADC,
            WDT,
            ..
        } = ctx.device;

        // Switch to the external HF oscillator for bluetooth
        // and start the low-power/low-frequency clock for RTCs
        let _clocks = Clocks::new(CLOCK).enable_ext_hfosc().start_lfclk();
        let gpio = gpio::p0::Parts::new(P0);
        let gpiote = Gpiote::new(GPIOTE);
        let ppi_channels = ppi::Parts::new(PPI);

        let watchdog = Watchdog::new(WDT);

        let mono = RtcMonotonic::new(RTC1, TIMER1, ppi_channels.ppi3).unwrap();
        let system_time = SystemTime::new();

        // TODO - disable RADIO for now
        // eventually make an enum for variants
        // UnInit(pac-devices)
        // Init(drivers)
        // ...
        // disabled on boot, enabled on-demand when the transport is needed
        // then reboot or button to turn it back off, only want the radio
        // on when needed
        RADIO.tasks_txen.write(|w| unsafe { w.bits(0) });
        RADIO.tasks_rxen.write(|w| unsafe { w.bits(0) });
        RADIO.tasks_stop.write(|w| unsafe { w.bits(1) });
        RADIO.tasks_disable.write(|w| unsafe { w.bits(1) });
        RADIO.tasks_bcstop.write(|w| unsafe { w.bits(1) });
        RADIO.events_disabled.write(|w| unsafe { w.bits(1) });
        RADIO.power.write(|w| unsafe { w.bits(0) });

        let mut delay = Timer::new(TIMER0);

        let motor_controller = MotorController::new(gpio.p0_16.into_push_pull_output(Level::High));

        // Button generates events on GPIOTE channel 0
        let button = Button::new(
            gpio.p0_15.into_push_pull_output(Level::High),
            gpio.p0_13.into_floating_input(),
            &gpiote.channel0(),
        );

        let bl0 = gpio.p0_14.into_push_pull_output(Level::High);
        let bl1 = gpio.p0_22.into_push_pull_output(Level::High);
        let bl2 = gpio.p0_23.into_push_pull_output(Level::High);
        let mut backlight = Backlight::new(bl0, bl1, bl2);
        backlight.set_brightness(Brightness::Off);

        let scl = gpio.p0_07.into_floating_input().degrade();
        let sda = gpio.p0_06.into_floating_input().degrade();
        let cst_rst = gpio.p0_10.into_push_pull_output(Level::High);
        let cst_int: cst816s::InterruptPin = gpio.p0_28.into_floating_input();
        let mut cst_twim = Twim::new(TWIM1, twim::Pins { scl, sda }, Frequency::K400);

        // The TWI device should work @ up to 400Khz but there is a HW bug which prevent it from
        // respecting correct timings. According to erratas heet, this magic value makes it run
        // at ~390Khz with correct timings.
        cst_twim.disable();
        unsafe {
            let twim = pac::TWIM0::ptr();
            (*twim)
                .frequency
                .write(|w| w.frequency().bits(cst816s::MAX_FREQUENCY));
        }
        cst_twim.enable();

        // CST816S generates events on channel 1
        let mut touch_controller = Cst816s::new(cst_twim, cst_rst, cst_int, &gpiote.channel1());
        while touch_controller.init(&mut delay).is_err() {
            delay.delay_ms(5_u32);
        }

        // PowerPresence pin generates events on GPIOTE channel 2
        let mut battery_controller = BatteryController::new(
            SAADC,
            gpio.p0_12.into_floating_input(),
            gpio.p0_19.into_floating_input(),
            gpio.p0_31.into_floating_input(),
            &gpiote.channel2(),
        );
        battery_controller.update();

        let spi_clk = gpio.p0_02.into_push_pull_output(Level::Low).degrade();
        let spi_mosi = gpio.p0_03.into_push_pull_output(Level::Low).degrade();
        let spi_miso = gpio.p0_04.into_floating_input().degrade();
        let spi_pins = spim::Pins {
            sck: spi_clk,
            miso: Some(spi_miso),
            mosi: Some(spi_mosi),
        };
        let display_spi = Spim::new(SPIM0, spi_pins, spim::Frequency::M8, spim::MODE_3, 0);

        // Display control
        let lcd_cs: LcdCsPin = gpio.p0_25.into_push_pull_output(Level::High);
        let lcd_dc: LcdDcPin = gpio.p0_18.into_push_pull_output(Level::Low);
        let lcd_rst: LcdResetPin = gpio.p0_26.into_push_pull_output(Level::Low);

        let di = SPIInterface::new(display_spi, lcd_dc, lcd_cs);
        let mut display = AnimatedSt7789::new(di, lcd_rst, display::WIDTH, display::HEIGHT);
        display.init(&mut delay).unwrap();

        display.clear(display::PixelFormat::BLACK).unwrap();

        let watch_face = WatchFace::new(ctx.local.font_styles, ctx.local.icons);

        watchdog_petter::spawn().unwrap();
        update_system_time::spawn().unwrap();
        poll_battery_voltage::spawn().unwrap();
        draw_screen::spawn().unwrap();
        ramp_on_backlight::spawn().unwrap();
        wakeup_display::spawn().unwrap();

        (
            Shared {
                display_state: AtomicDisplayAwakeState::new(false),
                display_sleep_timer: delay,
                button,
                system_time,
                backlight,
                display,
                battery_controller,
                motor_controller,
            },
            Local {
                gpiote,
                touch_controller,
                watchdog,
                watch_face,
            },
            init::Monotonics(mono),
        )
    }

    #[task(binds = GPIOTE, local = [gpiote], priority = 3)]
    fn gpiote_handler(ctx: gpiote_handler::Context) {
        if ctx.local.gpiote.channel0().is_event_triggered() {
            ctx.local.gpiote.channel0().reset_events();
            poll_button::spawn_after(Button::DEBOUNCE_MS).ok();
        }
        if ctx.local.gpiote.channel1().is_event_triggered() {
            ctx.local.gpiote.channel1().reset_events();
            touch_event::spawn().ok();
        }
        if ctx.local.gpiote.channel2().is_event_triggered() {
            ctx.local.gpiote.channel2().reset_events();
            poll_battery_io::spawn().ok();
        }
        if ctx.local.gpiote.port().is_event_triggered() {
            rprintln!("Unexpected interrupt from port event");
        }
    }

    #[task(local = [watchdog], shared = [button], priority = 4)]
    fn watchdog_petter(ctx: watchdog_petter::Context) {
        //let t = monotonics::now();
        //let t = Milliseconds::<u32>::try_from(t.duration_since_epoch()).unwrap();
        //rprintln!("wdt {:?}", t);

        // Holding the button down will eventually trip the watchdog and reset
        if !ctx.shared.button.is_pressed() {
            ctx.local.watchdog.pet();
        }
        watchdog_petter::spawn_after(Watchdog::PER_INTERVAL_MS).unwrap();
    }

    #[task(shared = [system_time], priority = 5)]
    fn update_system_time(ctx: update_system_time::Context) {
        let sys_time = ctx.shared.system_time;
        sys_time.update_time(monotonics::now());

        /*
        let t = monotonics::now();
        let d = t.duration_since_epoch();
        let ticks = d.integer();
        let ms = Milliseconds::<u32>::try_from(d).unwrap();
        rprintln!("t = {}, ms = {}", ticks, ms);

        ctx.shared.system_time.update_time(t);
        let dt = ctx.shared.system_time.date_time();
        let time = dt.time();
        rprintln!("ut {}", ctx.shared.system_time.uptime());
        rprintln!("{}:{}:{}", time.hour(), time.minute(), time.second());
        */

        update_system_time::spawn_after(Seconds(1_u32)).unwrap();
    }

    #[task(shared = [button], priority = 4)]
    fn poll_button(ctx: poll_button::Context) {
        if ctx.shared.button.is_pressed() {
            button_pressed::spawn().ok();
        }
    }

    #[task(shared = [&display_state], priority = 4)]
    fn button_pressed(ctx: button_pressed::Context) {
        // TODO - if already awake, then turn off disable
        let display_state = ctx.shared.display_state;
        if !display_state.is_awake() {
            wakeup_display::spawn().ok();
        }
    }

    #[task(local = [touch_controller], shared = [&display_state, display], priority = 5)]
    fn touch_event(ctx: touch_event::Context) {
        let touch_controller = ctx.local.touch_controller;
        let display_state = ctx.shared.display_state;
        let display = ctx.shared.display;

        if display_state.is_awake() {
            if let Some(touch_data) = touch_controller.read_touch_data() {
                rprintln!("{}", touch_data);
                match touch_data.gesture {
                    Some(Gesture::SlideUp) => display.set_refresh_direction(RefreshDirection::Up),
                    Some(Gesture::SlideDown) => {
                        display.set_refresh_direction(RefreshDirection::Down)
                    }
                    _ => (),
                }
            }
            wakeup_display::spawn().ok();
        }
    }

    #[task(shared = [backlight], priority = 6)]
    fn ramp_on_backlight(ctx: ramp_on_backlight::Context) {
        let backlight = ctx.shared.backlight;
        if backlight.brightness() != Brightness::L7 {
            backlight.brighter();
            ramp_on_backlight::spawn_after(Backlight::RAMP_INC_MS).unwrap();
        }
    }

    #[task(shared = [backlight], priority = 6)]
    fn ramp_off_backlight(ctx: ramp_off_backlight::Context) {
        let backlight = ctx.shared.backlight;
        if backlight.brightness() != Brightness::Off {
            backlight.darker();
            ramp_off_backlight::spawn_after(Backlight::RAMP_INC_MS).unwrap();
        }
    }

    #[task(shared = [&display_state, display_sleep_timer], priority = 6)]
    fn wakeup_display(ctx: wakeup_display::Context) {
        let display_state = ctx.shared.display_state;
        let display_sleep_timer = ctx.shared.display_sleep_timer;

        display_sleep_timer.start(DISPLAY_TIMEOUT_TIMER_TICKS);
        if !display_state.is_awake() {
            display_state.awaken();
            draw_screen::spawn().ok(); // backlight task is higher prio than display atm
            poll_display_timeout::spawn_after(DISPLAY_TIMEOUT_POLL_INTERVAL).ok();
            ramp_on_backlight::spawn().ok();
        }
    }

    #[task(shared = [&display_state, display_sleep_timer], priority = 6)]
    fn poll_display_timeout(ctx: poll_display_timeout::Context) {
        let display_state = ctx.shared.display_state;
        let display_sleep_timer = ctx.shared.display_sleep_timer;

        let timeout_expired = display_sleep_timer.wait().is_ok();
        if timeout_expired {
            let display_was_active = display_state.get_and_clear();
            if display_was_active {
                rprintln!("Display timeout");
                ramp_off_backlight::spawn().ok();
            }
        } else {
            poll_display_timeout::spawn_after(DISPLAY_TIMEOUT_POLL_INTERVAL).unwrap();
        }
    }

    // TODO - consider starting/resetting a timer here instead, and checking after it expires
    #[task(shared = [battery_controller], priority = 5)]
    fn poll_battery_io(ctx: poll_battery_io::Context) {
        if ctx.shared.battery_controller.update_charging_io() {
            rprintln!(
                "PBIO c {} v {} p {}",
                ctx.shared.battery_controller.is_charging(),
                ctx.shared.battery_controller.voltage(),
                ctx.shared.battery_controller.percent_remaining()
            );

            wakeup_display::spawn().ok();

            start_ring::spawn_after(
                BatteryController::POWER_PRESENCE_DEBOUNCE_MS,
                BatteryController::CHARGE_EVENT_RING_DURATION,
            )
            .ok();
        }
    }

    #[task(shared = [battery_controller], priority = 5)]
    fn poll_battery_voltage(ctx: poll_battery_voltage::Context) {
        ctx.shared.battery_controller.update_voltage();
        poll_battery_voltage::spawn_after(BatteryController::VOLTAGE_POLL_INTERVAL_MS).unwrap();
    }

    #[task(shared = [motor_controller], priority = 2)]
    fn start_ring(ctx: start_ring::Context, duration: Milliseconds<u32>) {
        if !ctx.shared.motor_controller.is_on() {
            rprintln!("start ring {}", duration);
            ctx.shared.motor_controller.on();
            stop_ring::spawn_after(duration).ok();
        }
    }

    #[task(shared = [motor_controller], priority = 2)]
    fn stop_ring(ctx: stop_ring::Context) {
        rprintln!("stop ring");
        ctx.shared.motor_controller.off();
    }

    #[task(
        local = [watch_face],
        shared = [&display_state, display, system_time, battery_controller],
        capacity = 2,
        priority = 5)
    ]
    fn draw_screen(ctx: draw_screen::Context) {
        let display = ctx.shared.display;
        let display_state = ctx.shared.display_state;

        // TODO - move this to after drawing so it can clear/draw over it
        display.update_animations().unwrap();

        if display_state.is_awake() {
            let screen = ctx.local.watch_face;

            let res = WatchFaceResources {
                sys_time: ctx.shared.system_time,
                bat_ctl: ctx.shared.battery_controller,
            };
            screen.update(&res).unwrap();
            screen.draw(display).unwrap();
            screen.clear_redraw();
        }

        draw_screen::spawn_after(SCREEN_REFRESH_INTERVAL).unwrap();
    }
}
