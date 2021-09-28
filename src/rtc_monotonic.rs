//! `Monotonic` implementation based on RTC peripheral
//!
//! The RTC provides TICK events to the TIMER task via ppi in
//! addition to handling the COMPARE events for the RTIC timer queue.

// TODO - revisit this, probably just use the RTC for ticks, 24 bits of ticks
// is probably fine
// use absolute val in set_compare no need to use rel duration
// https://rtic.rs/dev/book/en/by-example/tips_monotonic_impl.html

use crate::hal::{
    clocks::LFCLK_FREQ,
    ppi::{ConfigurablePpi, Ppi, Ppi3},
    rtc::{self, Rtc, RtcCompareReg, RtcInterrupt},
    timer,
};
use rtic::rtic_monotonic::{
    embedded_time::{clock::Error, fraction::Fraction},
    Clock, Instant, Monotonic,
};

pub const TICK_RATE_HZ: u32 = 1024;
/// Using TIMERx to count ticks, 32 bit
pub const MAX_TICKS: u32 = 0xFFFF_FFFF;

/// Example:
/// ```rust
/// #[monotonic(binds = TIMERx, default = true)]
/// type RtcMono = RtcMonotonic<RTCx, TIMERx>;
/// ```
pub struct RtcMonotonic<RTC: rtc::Instance, TIM: timer::Instance> {
    rtc: Rtc<RTC>,
    timer: TIM,
    _ppi: Ppi3,
}

impl<RTC, TIM> RtcMonotonic<RTC, TIM>
where
    RTC: rtc::Instance,
    TIM: timer::Instance,
{
    /// NOTE: LFCLK must be started before using the RTC peripheral
    pub fn new(rtc: RTC, timer: TIM, mut ppi: Ppi3) -> Result<Self, rtc::Error> {
        unsafe { rtc.tasks_stop.write(|w| w.bits(1)) };

        timer.timer_cancel();
        timer.disable_interrupt();
        timer.as_timer0().events_compare[0].reset();
        timer
            .as_timer0()
            .shorts
            .write(|w| w.compare0_clear().disabled().compare0_stop().disabled());
        timer
            .as_timer0()
            .prescaler
            .write(|w| unsafe { w.prescaler().bits(0) });
        timer.as_timer0().bitmode.write(|w| w.bitmode()._32bit());
        timer
            .as_timer0()
            .mode
            .write(|w| w.mode().low_power_counter());
        timer
            .as_timer0()
            .tasks_clear
            .write(|w| unsafe { w.bits(1) });

        // Route RTC TICK event to the TIMER counter task
        ppi.set_task_endpoint(&timer.as_timer0().tasks_count);
        ppi.set_event_endpoint(&rtc.events_tick);
        ppi.enable();

        // LFCLK_FREQ = 32768 Hz
        // fRTC = 32_768 / (prescaler + 1 )
        let prescaler = (LFCLK_FREQ / TICK_RATE_HZ) - 1;
        let mut rtc = Rtc::new(rtc, prescaler)?;

        // NOTE: the counter is started in the `reset` method
        rtc.disable_counter();
        rtc.disable_interrupt(RtcInterrupt::Compare0, None);
        rtc.disable_interrupt(RtcInterrupt::Tick, None);
        rtc.disable_event(RtcInterrupt::Compare0);
        rtc.disable_event(RtcInterrupt::Tick);
        rtc.clear_counter();

        Ok(RtcMonotonic {
            rtc,
            timer,
            _ppi: ppi,
        })
    }
}

impl<RTC, TIM> Clock for RtcMonotonic<RTC, TIM>
where
    RTC: rtc::Instance,
    TIM: timer::Instance,
{
    type T = u32;

    const SCALING_FACTOR: Fraction = Fraction::new(1, TICK_RATE_HZ);

    #[inline(always)]
    fn try_now(&self) -> Result<Instant<Self>, Error> {
        Ok(Instant::new(self.timer.read_counter()))
    }
}

impl<RTC, TIM> Monotonic for RtcMonotonic<RTC, TIM>
where
    RTC: rtc::Instance,
    TIM: timer::Instance,
{
    const DISABLE_INTERRUPT_ON_EMPTY_QUEUE: bool = true;

    unsafe fn reset(&mut self) {
        // TICK event routed to TIMER COUNTER task
        self.rtc.enable_event(RtcInterrupt::Tick);
        self.rtc.disable_interrupt(RtcInterrupt::Tick, None);

        self.rtc.set_compare(RtcCompareReg::Compare0, 0).unwrap();
        self.rtc.clear_counter();
        self.rtc.enable_event(RtcInterrupt::Compare0);
        self.rtc.enable_interrupt(RtcInterrupt::Compare0, None);
        self.rtc.enable_counter();
    }

    fn set_compare(&mut self, val: &Instant<Self>) {
        let now: Instant<Self> = Instant::new(self.timer.read_counter());

        let max = 0x00FF_FFFF;
        let dur = match val.checked_duration_since(&now) {
            None => {
                1 // In the past
            }
            Some(x) => max.min(x.integer()).max(1),
        };

        self.rtc.set_compare(RtcCompareReg::Compare0, dur).unwrap();
        self.rtc.clear_counter();
    }

    fn clear_compare_flag(&mut self) {
        if self.rtc.is_event_triggered(RtcInterrupt::Compare0) {
            self.rtc.reset_event(RtcInterrupt::Compare0);
        }
    }

    fn on_interrupt(&mut self) {
        if self.rtc.is_event_triggered(RtcInterrupt::Tick) {
            self.rtc.reset_event(RtcInterrupt::Tick);
        }
    }

    fn enable_timer(&mut self) {
        self.rtc.enable_event(RtcInterrupt::Compare0);
        self.rtc.enable_interrupt(RtcInterrupt::Compare0, None);
    }

    fn disable_timer(&mut self) {
        self.rtc.disable_interrupt(RtcInterrupt::Compare0, None);
        self.rtc.disable_event(RtcInterrupt::Compare0);
    }
}
