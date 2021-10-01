use crate::hal::{
    clocks::LFCLK_FREQ,
    pac::WDT,
    wdt::{self, Parts, Watchdog as HalWatchdog, WatchdogHandle},
};
use rtic::time::duration::Milliseconds;
use rtt_target::rprintln;

pub struct Watchdog {
    handle: WatchdogHandle<wdt::handles::Hdl0>,
}

impl Watchdog {
    pub const PERIOD: u32 = 3 * LFCLK_FREQ;
    pub const PER_INTERVAL_MS: Milliseconds<u32> = Milliseconds(500);

    pub fn new(wdt: WDT) -> Self {
        // In case the watchdog is already running, just spin and let it expire, since
        // we can't configure it anyway. This usually happens when we first program
        // the device and the watchdog was previously active
        let (handle,) = match HalWatchdog::try_new(wdt) {
            Ok(mut watchdog) => {
                // Set the watchdog to timeout after 5 seconds (in 32.768kHz ticks)
                watchdog.set_lfosc_ticks(Self::PERIOD);
                let Parts {
                    watchdog: _watchdog,
                    mut handles,
                } = watchdog.activate::<wdt::count::One>();
                handles.0.pet();
                handles
            }
            Err(wdt) => match HalWatchdog::try_recover::<wdt::count::One>(wdt) {
                Ok(Parts { mut handles, .. }) => {
                    rprintln!("Watchdog already active, recovering");
                    handles.0.pet();
                    handles
                }
                Err(_wdt) => {
                    rprintln!("Watchdog already set, can't recovery, resetting");
                    loop {
                        cortex_m::asm::nop();
                    }
                }
            },
        };
        Watchdog { handle }
    }

    pub fn pet(&mut self) {
        self.handle.pet();
    }
}
