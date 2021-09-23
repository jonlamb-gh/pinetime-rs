#![no_main]
#![no_std]

// helpful links
//
// https://github.com/JF002/InfiniTime
// https://lupyuen.github.io/pinetime-rust-mynewt/articles/timesync
//
// https://github.com/JF002/InfiniTime/pull/595
//
// docs
// https://github.com/JF002/InfiniTime/blob/develop/bootloader/README.md
// https://github.com/JF002/InfiniTime/blob/develop/doc/MemoryAnalysis.md
//
// https://github.com/nrf-rs/nrf-hal/blob/master/examples/rtc-demo/src/main.rs
// https://github.com/nrf-rs/nrf-hal/tree/master/examples/rtic-demo
// https://github.com/almindor/st7789-examples/tree/master/examples
//
// https://rtic.rs/dev/book/en/by-example/app.html
//
// https://docs.rs/dwt-systick-monotonic/0.1.0-alpha.3/dwt_systick_monotonic/struct.DwtSystick.html
//
// rtic dma example
// https://github.com/nrf-rs/nrf-hal/blob/master/examples/twis-dma-demo/src/main.rs
//
// TODO
// some sort of error handling pattern

use nrf52832_hal as hal;
use panic_rtt_target as _;

mod rtc_monotonic;

#[rtic::app(device = crate::hal::pac, peripherals = true, dispatchers = [SWI0_EGU0])]
mod app {
    use crate::{hal, rtc_monotonic};
    use display_interface_spi::SPIInterfaceNoCS;
    use embedded_graphics::prelude::*;
    use hal::{
        clocks::Clocks,
        gpio::{self, Level, Output, PushPull},
        gpiote::Gpiote,
        pac, ppi,
        prelude::*,
        spim::{self, Spim},
        timer::Timer,
        twim::{self, Frequency, Twim},
    };
    use pinetime_lib::{
        backlight::{Backlight, Brightness},
        battery_controller::BatteryController,
        button::Button,
        cst816s::{self, Cst816s},
        display,
        resources::FontStyles,
    };
    use rtc_monotonic::RtcMonotonic;
    use rtic::time::duration::Milliseconds;
    use rtt_target::{rprintln, rtt_init_print};
    use st7789::{Orientation, ST7789};

    // TODO - move drawing to module
    // probably a "watchface" thing
    use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};

    const TICK_RATE_HZ: u32 = 1024;

    #[monotonic(binds = RTC1, default = true)]
    type RtcMono = RtcMonotonic<pac::RTC1, pac::TIMER1, TICK_RATE_HZ>;

    #[shared]
    struct Shared {
        font_styles: FontStyles,
        // icons
        delay: Timer<pac::TIMER0>,

        #[lock_free]
        display: ST7789<
            SPIInterfaceNoCS<Spim<pac::SPIM1>, gpio::p0::P0_18<Output<PushPull>>>,
            gpio::p0::P0_26<Output<PushPull>>,
        >,
    }

    #[local]
    struct Local<'a> {
        gpiote: Gpiote,
        _button: Button,
        backlight: Backlight,
        touch_controller: Cst816s<pac::TWIM0>,
        battery_controller: BatteryController,
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
            ..
        } = ctx.device;

        // Switch to the external HF oscillator for bluetooth
        // and start the low-power/low-frequency clock for RTCs
        let _clocks = Clocks::new(CLOCK).enable_ext_hfosc().start_lfclk();
        let gpio = gpio::p0::Parts::new(P0);
        let gpiote = Gpiote::new(GPIOTE);
        let ppi_channels = ppi::Parts::new(PPI);

        // TODO - watchdog

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

        // TODO - in release builds, getting Error::AddressNack
        // probably after controller goes into sleep mode, need to wait for first wakeup interrupt
        // to init
        //
        // also setup the watchdog early on, maybe loop here a few times
        // CST816S generates events on channel 1
        let mut touch_controller = Cst816s::new(cst_twim, cst_rst, cst_int, &gpiote.channel1());
        while touch_controller.init(&mut delay).is_err() {
            delay.delay_ms(5_u32);
        }

        // PowerPresence pin generates events on GPIOTE channel 2
        let battery_controller = BatteryController::new(
            SAADC,
            gpio.p0_12.into_floating_input(),
            gpio.p0_19.into_floating_input(),
            gpio.p0_31.into_floating_input(),
            &gpiote.channel2(),
        );

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
        let mut lcd_cs: display::LcdCsPin = gpio.p0_25.into_push_pull_output(Level::Low);
        let lcd_dc: display::LcdDcPin = gpio.p0_18.into_push_pull_output(Level::Low);
        let lcd_rst: display::LcdResetPin = gpio.p0_26.into_push_pull_output(Level::Low);

        // Hold CS low while driving the display
        lcd_cs.set_low().unwrap();

        let di = SPIInterfaceNoCS::new(display_spi, lcd_dc);
        let mut display = ST7789::new(di, lcd_rst, display::WIDTH, display::HEIGHT);
        display.init(&mut delay).unwrap();
        display.set_orientation(Orientation::Portrait).unwrap();

        display.clear(display::PixelFormat::BLACK).unwrap();

        poll_battery_controller::spawn().unwrap();
        update_display::spawn().unwrap();

        (
            Shared {
                font_styles: FontStyles::default(),
                delay,
                display,
            },
            Local {
                gpiote,
                backlight,
                _button: button,
                touch_controller,
                battery_controller,
            },
            init::Monotonics(mono),
        )
    }

    #[task(binds = GPIOTE, local = [gpiote])]
    fn gpiote_handler(ctx: gpiote_handler::Context) {
        if ctx.local.gpiote.channel0().is_event_triggered() {
            ctx.local.gpiote.channel0().reset_events();
            button_pressed::spawn().unwrap();
        }
        if ctx.local.gpiote.channel1().is_event_triggered() {
            ctx.local.gpiote.channel1().reset_events();
            touch_event::spawn().unwrap();
        }
        if ctx.local.gpiote.channel2().is_event_triggered() {
            ctx.local.gpiote.channel2().reset_events();
            poll_battery_controller::spawn().unwrap();
        }
        if ctx.local.gpiote.port().is_event_triggered() {
            rprintln!("Unexpected interrupt from port event");
        }
    }

    #[task(local = [backlight])]
    fn button_pressed(ctx: button_pressed::Context) {
        ctx.local.backlight.brighter();
        rprintln!("button pressed b={}", ctx.local.backlight.brightness());
        if ctx.local.backlight.brightness() == Brightness::L7 {
            ctx.local.backlight.set_brightness(Brightness::Off);
        }
    }

    #[task(local = [touch_controller])]
    fn touch_event(ctx: touch_event::Context) {
        if let Some(touch_data) = ctx.local.touch_controller.read_touch_data() {
            rprintln!("{}", touch_data);
        }
    }

    // TODO rm cap once a handler task is made, just for hacking on power pin events
    #[task(local = [battery_controller], capacity = 4)]
    fn poll_battery_controller(ctx: poll_battery_controller::Context) {
        if ctx.local.battery_controller.update() {
            rprintln!(
                "BAT c {} p {} v {} p {}",
                ctx.local.battery_controller.charging(),
                ctx.local.battery_controller.power_present(),
                ctx.local.battery_controller.voltage(),
                ctx.local.battery_controller.percent_remaining()
            );
        }
        poll_battery_controller::spawn_after(Milliseconds(5 * 1024_u32)).unwrap();
    }

    #[task(shared = [&font_styles, display])]
    fn update_display(ctx: update_display::Context) {
        rprintln!("display");
        let text = "12:12";
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
        //update_display::spawn_after(Milliseconds(512_u32)).unwrap();
    }
}
