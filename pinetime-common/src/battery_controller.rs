use core::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(transparent)]
pub struct MilliVolts(pub u16);

impl fmt::Display for MilliVolts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} mV", self.0)
    }
}

pub trait BatteryControllerExt {
    fn is_charging(&self) -> bool;

    fn voltage(&self) -> MilliVolts;

    fn percent_remaining(&self) -> u8;
}
