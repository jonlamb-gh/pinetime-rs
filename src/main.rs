#![no_main]
#![no_std]

// helpful links
//
// https://github.com/JF002/InfiniTime
// https://github.com/JF002/InfiniTime/blob/develop/src/components/datetime/DateTimeController.cpp
// https://lupyuen.github.io/pinetime-rust-mynewt/articles/timesync
// https://github.com/JF002/InfiniTime/blob/136d4bb85e36777f0f9877fd065476ba1c02ca90/src/FreeRTOS/port_cmsis_systick.c
//
// https://github.com/JF002/InfiniTime/pull/595
//
// rtic dma example
// https://github.com/nrf-rs/nrf-hal/blob/master/examples/twis-dma-demo/src/main.rs
//
// TODO
// some sort of error handling pattern / resource and priority management

use nrf52832_hal as hal;
use panic_rtt_target as _;

mod rtc_monotonic;
mod system_time;

#[rtic::app(device = crate::hal::pac, peripherals = true, dispatchers = [SWI0_EGU0, SWI1_EGU1, SWI2_EGU2, SWI3_EGU3])]
mod app {
    use crate::{hal, rtc_monotonic};
    use core::convert::TryFrom;
    use display_interface_spi::SPIInterfaceNoCS;
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
    use pinetime_drivers::{
        backlight::{Backlight, Brightness},
        battery_controller::BatteryController,
        button::Button,
        cst816s::{self, Cst816s},
        lcd::{LcdCsPin, LcdDcPin, LcdResetPin},
        motor_controller::MotorController,
        watchdog::Watchdog,
    };
    use pinetime_graphics::{display, embedded_graphics::prelude::*, font_styles::FontStyles};
    use rtc_monotonic::{RtcMonotonic, TICK_RATE_HZ};
    use rtic::time::duration::{Milliseconds, Seconds};
    use rtt_target::{rprintln, rtt_init_print};
    use st7789::{Orientation, ST7789};

    // TODO - move drawing to module
    // probably a "watchface" thing
    use pinetime_graphics::embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};

    //#[monotonic(binds = RTC1, default = true, priority = 6)]
    #[monotonic(binds = RTC1, default = true)]
    type RtcMono = RtcMonotonic<pac::RTC1, pac::TIMER1, TICK_RATE_HZ>;

    #[shared]
    struct Shared {
        font_styles: FontStyles,
        // icons
        _delay: Timer<pac::TIMER0>,

        // Move to local, take a DisplayEvent arg, other tasks can send events to it
        #[lock_free]
        display: ST7789<SPIInterfaceNoCS<Spim<pac::SPIM1>, LcdDcPin>, LcdResetPin>,

        #[lock_free]
        battery_controller: BatteryController,

        #[lock_free]
        motor_controller: MotorController,
    }

    #[local]
    struct Local<'a> {
        gpiote: Gpiote,
        button: Button,
        backlight: Backlight,
        touch_controller: Cst816s<pac::TWIM0>,
        watchdog: Watchdog,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        rprintln!("Initializing");

        let hal::pac::Peripherals {
            CLOCK,
            P0,
            SPIM1,
            TIMER0,
            TIMER1,
            RTC1,
            PPI,
            GPIOTE,
            TWIM0,
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

        // TODO - disable RADIO for now
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
        backlight.set_brightness(Brightness::L7);

        let scl = gpio.p0_07.into_floating_input().degrade();
        let sda = gpio.p0_06.into_floating_input().degrade();
        let cst_rst = gpio.p0_10.into_push_pull_output(Level::High);
        let cst_int: cst816s::InterruptPin = gpio.p0_28.into_floating_input();
        let mut cst_twim = Twim::new(TWIM0, twim::Pins { scl, sda }, Frequency::K400);

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
        let display_spi = Spim::new(SPIM1, spi_pins, spim::Frequency::M8, spim::MODE_3, 0);

        // Display control
        let mut lcd_cs: LcdCsPin = gpio.p0_25.into_push_pull_output(Level::Low);
        let lcd_dc: LcdDcPin = gpio.p0_18.into_push_pull_output(Level::Low);
        let lcd_rst: LcdResetPin = gpio.p0_26.into_push_pull_output(Level::Low);

        // Hold CS low while driving the display
        lcd_cs.set_low().unwrap();

        let di = SPIInterfaceNoCS::new(display_spi, lcd_dc);
        let mut display = ST7789::new(di, lcd_rst, display::WIDTH, display::HEIGHT);
        display.init(&mut delay).unwrap();
        display.set_orientation(Orientation::Portrait).unwrap();

        display.clear(display::PixelFormat::BLACK).unwrap();

        watchdog_petter::spawn().unwrap();
        poll_battery_status::spawn().unwrap();
        update_display::spawn().unwrap();

        (
            Shared {
                font_styles: FontStyles::default(),
                _delay: delay,
                display,
                battery_controller,
                motor_controller,
            },
            Local {
                gpiote,
                backlight,
                button,
                touch_controller,
                watchdog,
            },
            init::Monotonics(mono),
        )
    }

    #[task(local = [watchdog], priority = 4)]
    fn watchdog_petter(ctx: watchdog_petter::Context) {
        ctx.local.watchdog.pet();
        watchdog_petter::spawn_after(Seconds(1_u32)).unwrap();
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

    #[task(local = [button])]
    fn poll_button(ctx: poll_button::Context) {
        if ctx.local.button.is_pressed() {
            button_pressed::spawn().ok();
        }
    }

    #[task(local = [backlight])]
    fn button_pressed(ctx: button_pressed::Context) {
        if ctx.local.backlight.brightness() == Brightness::L7 {
            ctx.local.backlight.set_brightness(Brightness::Off);
        } else {
            ctx.local.backlight.brighter();
        }
        rprintln!("button pressed b={}", ctx.local.backlight.brightness());
    }

    #[task(local = [touch_controller])]
    fn touch_event(ctx: touch_event::Context) {
        if let Some(touch_data) = ctx.local.touch_controller.read_touch_data() {
            rprintln!("{}", touch_data);
        }
    }

    // TODO - consider starting/resetting a timer here instead, and checking after it expires
    #[task(shared = [battery_controller])]
    fn poll_battery_io(ctx: poll_battery_io::Context) {
        if ctx.shared.battery_controller.update_charging_io() {
            rprintln!(
                "PBIO c {} v {} p {}",
                ctx.shared.battery_controller.is_charging(),
                ctx.shared.battery_controller.voltage(),
                ctx.shared.battery_controller.percent_remaining()
            );
            start_ring::spawn_after(
                BatteryController::POWER_PRESENCE_DEBOUNCE_MS,
                BatteryController::CHARGE_EVENT_RING_DURATION,
            )
            .ok();
        }
    }

    #[task(shared = [battery_controller])]
    fn poll_battery_status(ctx: poll_battery_status::Context) {
        let (charging_changed, voltage_changed) = ctx.shared.battery_controller.update();

        if charging_changed || voltage_changed {
            rprintln!(
                "PBS c {} v {} p {}",
                ctx.shared.battery_controller.is_charging(),
                ctx.shared.battery_controller.voltage(),
                ctx.shared.battery_controller.percent_remaining()
            );
        }

        poll_battery_status::spawn_after(Milliseconds(5 * 1024_u32)).unwrap();
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

    #[task(shared = [&font_styles, display], priority = 5)]
    fn update_display(ctx: update_display::Context) {
        let t = monotonics::RtcMono::now();
        let t = Milliseconds::<u32>::try_from(t.duration_since_epoch()).unwrap();
        rprintln!("display at {:?}", t);

        let text = "12:10";
        let font_style = ctx.shared.font_styles.watchface_time_style;
        let text_style = TextStyleBuilder::new()
            .baseline(Baseline::Alphabetic)
            .alignment(Alignment::Center)
            .build();
        let pos_x = (display::WIDTH / 2) as i32;
        let pos_y = (display::HEIGHT / 2) as i32;
        Text::with_text_style(text, Point::new(pos_x, pos_y), font_style, text_style)
            .draw(ctx.shared.display)
            .unwrap();

        let text = "FRI 24 SEP 2021";
        let font_style = ctx.shared.font_styles.watchface_date_style;
        let text_style = TextStyleBuilder::new()
            .baseline(Baseline::Alphabetic)
            .alignment(Alignment::Center)
            .build();
        let pos_x = (display::WIDTH / 2) as i32;
        let pos_y = (display::HEIGHT / 2) as i32 + 50;
        Text::with_text_style(text, Point::new(pos_x, pos_y), font_style, text_style)
            .draw(ctx.shared.display)
            .unwrap();

        //update_display::spawn_after(Milliseconds(1000_u32)).unwrap();
    }
}
