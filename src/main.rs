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
//
// rtic dma example
// https://github.com/nrf-rs/nrf-hal/blob/master/examples/twis-dma-demo/src/main.rs
//
// TODO
// err_derive patterns

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
        gpio::{self, Floating, Input, Level, Output, Pin, PushPull},
        gpiote::{Gpiote, GpioteInputPin},
        pac, ppi,
        prelude::*,
        spim::{self, Spim},
        timer::Timer,
        twim::{self, Frequency, Twim},
    };
    use pinetime_lib::{
        backlight::{Backlight, Brightness},
        cst816s::{self, Cst816s},
        display,
        resources::FontStyles,
    };
    use rtc_monotonic::RtcMonotonic;
    use rtic::time::duration::Milliseconds;
    use rtt_target::{rprintln, rtt_init_print};
    use st7789::{Orientation, ST7789};

    // TODO - move drawing to module
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
    struct Local {
        gpiote: Gpiote,
        button: Pin<Input<Floating>>,
        backlight: Backlight,
        touch_controller: Cst816s<pac::TWIM0>,
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
            ..
        } = ctx.device;

        // Switch to the external HF oscillator for bluetooth
        // and start the low-power/low-frequency clock for RTCs
        let _clocks = Clocks::new(CLOCK).enable_ext_hfosc().start_lfclk();
        let gpio = gpio::p0::Parts::new(P0);
        let gpiote = Gpiote::new(GPIOTE);
        let ppi_channels = ppi::Parts::new(PPI);

        let mono = RtcMonotonic::new(RTC1, TIMER1, ppi_channels.ppi3).unwrap();

        // TODO - disable RADIO for now

        let mut delay = Timer::new(TIMER0);

        gpio.p0_15.into_push_pull_output(Level::High);
        let button = gpio.p0_13.into_floating_input().degrade();

        let bl0 = gpio.p0_14.into_push_pull_output(Level::High).degrade();
        let bl1 = gpio.p0_22.into_push_pull_output(Level::High).degrade();
        let bl2 = gpio.p0_23.into_push_pull_output(Level::High).degrade();
        let mut backlight = Backlight::new(bl0, bl1, bl2);
        backlight.set_brightness(Brightness::Off);

        let scl = gpio.p0_07.into_floating_input().degrade();
        let sda = gpio.p0_06.into_floating_input().degrade();
        let cst_rst: cst816s::ResetPin = gpio.p0_10.into_push_pull_output(Level::High);
        let cst_int: cst816s::IntPin = gpio.p0_28.into_floating_input();
        let mut cst_twim = Twim::new(TWIM0, twim::Pins { scl, sda }, Frequency::K400);

        // The TWI device should work @ up to 400Khz but there is a HW bug which prevent it from
        // respecting correct timings. According to erratas heet, this magic value makes it run
        // at ~390Khz with correct timings.
        cst_twim.disable();
        unsafe {
            let twim = pac::TWIM0::ptr();
            (*twim).frequency.write(|w| w.frequency().bits(0x06200000));
        }
        cst_twim.enable();

        let mut touch_controller = Cst816s::new(cst_twim, cst_rst.degrade());
        touch_controller.init(&mut delay).unwrap();

        // Setup GPIO events and interrupts
        // Button generates event on channel 0
        gpiote
            .channel0()
            .input_pin(&button)
            .lo_to_hi()
            .enable_interrupt();
        // CST816S generates event on channel 1
        gpiote
            .channel1()
            .input_pin(&cst_int.degrade())
            .lo_to_hi()
            .enable_interrupt();

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
        let mut lcd_cs = gpio.p0_25.into_push_pull_output(Level::Low);
        let lcd_dc = gpio.p0_18.into_push_pull_output(Level::Low);
        let lcd_rst = gpio.p0_26.into_push_pull_output(Level::Low);

        // Hold CS low while driving the display
        lcd_cs.set_low().unwrap();

        let di = SPIInterfaceNoCS::new(display_spi, lcd_dc);
        let mut display = ST7789::new(di, lcd_rst, display::WIDTH, display::HEIGHT);
        display.init(&mut delay).unwrap();
        display.set_orientation(Orientation::Portrait).unwrap();

        display.clear(display::PixelFormat::BLACK).unwrap();

        update_display::spawn().unwrap();
        //clock_test::spawn_after(Milliseconds(512_u32)).unwrap();

        (
            Shared {
                font_styles: FontStyles::default(),
                delay,
                display,
            },
            Local {
                gpiote,
                button,
                backlight,
                touch_controller,
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

    #[task(binds = GPIOTE, local = [gpiote])]
    fn gpiote_handler(ctx: gpiote_handler::Context) {
        //rprintln!("GPIOTE event");
        if ctx.local.gpiote.channel0().is_event_triggered() {
            ctx.local.gpiote.channel0().reset_events();
            //rprintln!("Interrupt from channel 0 event");
            button_pressed::spawn().unwrap();
        }
        if ctx.local.gpiote.channel1().is_event_triggered() {
            ctx.local.gpiote.channel1().reset_events();
            //rprintln!("Interrupt from channel 1 event");
            touch_event::spawn().unwrap();
        }
        if ctx.local.gpiote.port().is_event_triggered() {
            rprintln!("Interrupt from port event");
        }
        // Reset all events
        //ctx.local.gpiote.reset_events();
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
