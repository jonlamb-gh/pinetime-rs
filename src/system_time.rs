//! System time managing seconds since epoch

use crate::hal::{rtc, timer};
use crate::rtc_monotonic::{RtcMonotonic, MAX_TICKS, TICK_RATE_HZ};
use chrono::{Duration, NaiveDateTime};
use rtic::time::{duration::Seconds, Instant};

pub struct SystemTime<RTC: rtc::Instance, TIM: timer::Instance> {
    uptime: Seconds,
    last_clock_instant: Instant<RtcMonotonic<RTC, TIM>>,
    date_time: NaiveDateTime,
}

impl<RTC, TIM> SystemTime<RTC, TIM>
where
    RTC: rtc::Instance,
    TIM: timer::Instance,
{
    pub fn new() -> Self {
        SystemTime {
            uptime: Seconds::new(0),
            last_clock_instant: Instant::new(0),
            date_time: NaiveDateTime::from_timestamp(0, 0),
        }
    }

    pub fn update_time(&mut self, now: Instant<RtcMonotonic<RTC, TIM>>) {
        let ticks = now.duration_since_epoch().integer();
        let prev_ticks = self.last_clock_instant.duration_since_epoch().integer();

        let tick_delta = if ticks < prev_ticks {
            (MAX_TICKS - prev_ticks) + (ticks + 1)
        } else {
            ticks - prev_ticks
        };

        let corrected_tick_delta = tick_delta / TICK_RATE_HZ;
        let rest = tick_delta - (corrected_tick_delta * TICK_RATE_HZ);
        let last_clock_ticks = if ticks >= rest {
            ticks - rest
        } else {
            MAX_TICKS - (rest - ticks)
        };

        self.last_clock_instant = Instant::new(last_clock_ticks);

        let sec = Seconds::new(corrected_tick_delta);
        self.uptime = self.uptime + sec;

        self.date_time += Duration::from_std(core::time::Duration::from_secs(sec.0 as _)).unwrap();
    }

    pub fn uptime(&self) -> Seconds {
        self.uptime
    }

    pub fn date_time(&self) -> &NaiveDateTime {
        &self.date_time
    }
}
