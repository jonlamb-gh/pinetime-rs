use crate::hal::{
    gpio::{Output, Pin, PushPull},
    prelude::OutputPin,
};
use core::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Brightness {
    Off,
    L1,
    L2,
    L3,
    L4,
    L5,
    L6,
    L7,
}

impl Default for Brightness {
    fn default() -> Self {
        Brightness::brightest()
    }
}

impl Brightness {
    fn as_u8(self) -> u8 {
        use Brightness::*;
        match self {
            Off => 0,
            L1 => 1,
            L2 => 2,
            L3 => 3,
            L4 => 4,
            L5 => 5,
            L6 => 6,
            L7 => 7,
        }
    }

    pub fn brightest() -> Self {
        Brightness::L7
    }

    pub fn dimmest() -> Self {
        Brightness::L1
    }

    pub fn brighter(self) -> Self {
        use Brightness::*;
        match self {
            Off => L1,
            L1 => L2,
            L2 => L3,
            L3 => L4,
            L4 => L5,
            L5 => L6,
            L6 => L7,
            L7 => L7,
        }
    }

    pub fn darker(self) -> Self {
        use Brightness::*;
        match self {
            Off => Off,
            L1 => Off,
            L2 => L1,
            L3 => L2,
            L4 => L3,
            L5 => L4,
            L6 => L5,
            L7 => Off,
        }
    }
}

impl fmt::Display for Brightness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Brightness::*;
        let s = match self {
            Off => "Off",
            L1 => "1",
            L2 => "2",
            L3 => "3",
            L4 => "4",
            L5 => "5",
            L6 => "6",
            L7 => "Max",
        };
        write!(f, "{}", s)
    }
}

pub struct Backlight {
    low: Pin<Output<PushPull>>,
    mid: Pin<Output<PushPull>>,
    high: Pin<Output<PushPull>>,
    brightness: Brightness,
}

impl Backlight {
    pub fn new(
        low: Pin<Output<PushPull>>,
        mid: Pin<Output<PushPull>>,
        high: Pin<Output<PushPull>>,
    ) -> Self {
        let mut backlight = Backlight {
            low,
            mid,
            high,
            brightness: Brightness::default(),
        };
        backlight.set_brightness(backlight.brightness);
        backlight
    }

    pub fn off(&mut self) {
        self.set_brightness(Brightness::Off);
    }

    pub fn brighter(&mut self) {
        self.set_brightness(self.brightness.brighter());
    }

    pub fn darker(&mut self) {
        self.set_brightness(self.brightness.darker());
    }

    pub fn brightness(&self) -> Brightness {
        self.brightness
    }

    pub fn set_brightness(&mut self, brightness: Brightness) {
        let b = brightness.as_u8();
        if b & 0x01 > 0 {
            self.low.set_low().unwrap();
        } else {
            self.low.set_high().unwrap();
        }
        if b & 0x02 > 0 {
            self.mid.set_low().unwrap();
        } else {
            self.mid.set_high().unwrap();
        }
        if b & 0x04 > 0 {
            self.high.set_low().unwrap();
        } else {
            self.high.set_high().unwrap();
        }
        self.brightness = brightness;
    }
}
