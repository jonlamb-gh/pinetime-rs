use crate::hal::gpio::{p0, Output, PushPull};

pub type LcdCsPin = p0::P0_25<Output<PushPull>>;
pub type LcdDcPin = p0::P0_18<Output<PushPull>>;
pub type LcdResetPin = p0::P0_26<Output<PushPull>>;
