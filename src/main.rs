#![no_main]
#![no_std]

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

use nrf52832_hal as hal;
use panic_rtt_target as _;

mod rtc_monotonic;

#[rtic::app(device = crate::hal::pac, peripherals = true, dispatchers = [SWI0_EGU0])]
mod app {
    use super::{hal, rtc_monotonic};
    use debouncr::{debounce_6, Debouncer, Edge, Repeat6};
    use display_interface_spi::SPIInterfaceNoCS;
    use embedded_graphics::prelude::*;
    use hal::{
        clocks::{Clocks, LfOscConfiguration, HFCLK_FREQ, LFCLK_FREQ},
        gpio::{self, Floating, Input, Level, Output, Pin, PushPull},
        pac, ppi,
        prelude::*,
        rtc::RtcInterrupt,
        spim::{self, Spim},
        timer::{self, Timer},
    };
    use pinetime_lib::{display, resources::Fonts};
    use rtc_monotonic::RtcMonotonic;
    use rtic::time::duration::{Milliseconds, Seconds};
    use rtt_target::{rprintln, rtt_init_print};
    use st7789::{Orientation, ST7789};

    // TODO - move drawing to module
    use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};

    const TICK_RATE_HZ: u32 = 1024;

    #[monotonic(binds = RTC1, default = true)]
    type RtcMono = RtcMonotonic<pac::RTC1, pac::TIMER1, TICK_RATE_HZ>;

    #[shared]
    struct Shared {
        fonts: Fonts,
        // icons
        #[lock_free]
        display: ST7789<
            SPIInterfaceNoCS<Spim<pac::SPIM1>, gpio::p0::P0_18<Output<PushPull>>>,
            gpio::p0::P0_26<Output<PushPull>>,
        >,
    }

    #[local]
    struct Local {
        button: Pin<Input<Floating>>,
        button_debouncer: Debouncer<u8, Repeat6>,
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
            ..
        } = ctx.device;

        // Switch to the external HF oscillator for bluetooth
        // and start the low-power/low-frequency clock for RTCs
        let clocks = Clocks::new(CLOCK).enable_ext_hfosc().start_lfclk();
        let gpio = gpio::p0::Parts::new(P0);
        let ppi_channels = ppi::Parts::new(PPI);

        let mono = RtcMonotonic::new(RTC1, TIMER1, ppi_channels.ppi3).unwrap();

        // TODO - disable RADIO for now

        let mut delay = Timer::new(TIMER0);

        gpio.p0_15.into_push_pull_output(Level::High);
        let button = gpio.p0_13.into_floating_input().degrade();

        // TODO backlight
        let mut bl0 = gpio.p0_14.into_push_pull_output(Level::High).degrade();
        let mut bl1 = gpio.p0_22.into_push_pull_output(Level::High).degrade();
        let mut bl2 = gpio.p0_23.into_push_pull_output(Level::High).degrade();
        //bl0.set_low().unwrap();
        //bl1.set_low().unwrap();
        //bl2.set_low().unwrap();

        let spi_clk = gpio.p0_02.into_push_pull_output(Level::Low).degrade();
        let spi_mosi = gpio.p0_03.into_push_pull_output(Level::Low).degrade();
        let spi_miso = gpio.p0_04.into_floating_input().degrade();
        let spi_pins = spim::Pins {
            sck: spi_clk,
            miso: Some(spi_miso),
            mosi: Some(spi_mosi),
        };

        let mut lcd_cs = gpio.p0_25.into_push_pull_output(Level::Low);
        let lcd_dc = gpio.p0_18.into_push_pull_output(Level::Low);
        let lcd_rst = gpio.p0_26.into_push_pull_output(Level::Low);

        let spi = Spim::new(SPIM1, spi_pins, spim::Frequency::M8, spim::MODE_3, 0);

        // Hold CS low while driving the display
        lcd_cs.set_low().unwrap();

        let di = SPIInterfaceNoCS::new(spi, lcd_dc);
        let mut display = ST7789::new(di, lcd_rst, display::WIDTH, display::HEIGHT);
        display.init(&mut delay).unwrap();
        display.set_orientation(Orientation::Portrait).unwrap();

        display.clear(display::PixelFormat::BLACK).unwrap();

        //poll_button::spawn().unwrap();
        //update_display::spawn().unwrap();
        clock_test::spawn_after(Milliseconds(512_u32)).unwrap();

        (
            Shared {
                fonts: Fonts::default(),
                display,
            },
            Local {
                button,
                button_debouncer: debounce_6(false),
            },
            init::Monotonics(mono),
        )
    }

    // TODO - don't need for now, use RTCx for scheduler, low-power mode when in idle
    /*
    #[idle]
    fn idle(_: idle::Context) -> ! {
        rprintln!("idle");

        loop {
            cortex_m::asm::nop();
        }
    }
    */

    /*
    #[task(local = [button, button_debouncer])]
    fn poll_button(ctx: poll_button::Context) {
        let pressed = ctx.local.button.is_high().unwrap();
        let edge = ctx.local.button_debouncer.update(pressed);

        if edge == Some(Edge::Rising) {
            // TODO
            button_pressed::spawn().unwrap();
        }

        poll_button::spawn_after(Milliseconds(2_u32)).unwrap();
    }

    #[task]
    fn button_pressed(_ctx: button_pressed::Context) {
        rprintln!("button pressed");
    }
    */

    #[task]
    fn clock_test(ctx: clock_test::Context) {
        rprintln!("TICK");
        clock_test::spawn_after(Milliseconds(512_u32)).unwrap();
    }

    /*
    #[task(shared = [&fonts, display])]
    fn update_display(ctx: update_display::Context) {
        rprintln!("display");
        let text = "12:12";
        let font_style = ctx.shared.fonts.watchface_time_style;
        let text_style = TextStyleBuilder::new()
            .baseline(Baseline::Alphabetic)
            .alignment(Alignment::Center)
            .build();
        let pos_x = (display::WIDTH / 2) as i32;
        let pos_y = (display::HEIGHT / 2) as i32;
        Text::with_text_style(text, Point::new(pos_x, pos_y), font_style, text_style)
            .draw(ctx.shared.display)
            .unwrap();
        //update_display::spawn_after(Milliseconds(500_u32)).unwrap();
    }
    */
}
