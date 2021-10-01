// TODO
// - pins/SPI type aliases
// - need to figure out a shared-bus strategy, SPIM used for both flash and ST7789
// https://github.com/jonas-schievink/spi-memory/issues/27
// https://crates.io/crates/shared-bus-rtic
// https://crates.io/crates/shared-bus

/*
use crate::hal::prelude::{OutputPin, _embedded_hal_blocking_spi_Transfer as Transfer};
use core::fmt;


#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Identification {
    pub manufacturer: u8,
    pub typ: u8,
    pub density: u8,
}

impl fmt::Display for Identification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Identification(manufacturer=0x{:02X}, type=0x{:02X}, density=0x{:02X}",
            self.manufacturer, self.typ, self.density
        )
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Opcode {
    PageProg = 0x02,
    Read = 0x03,
    ReadStatus = 0x05,
    WriteEnable = 0x06,
    ReadConfigurationRegister = 0x15,
    SectorErase = 0x20,
    ReadSecurityRegister = 0x2B,
    ReadIdentification = 0x9F,
    ReleaseFromDeepPowerDown = 0xAB,
    DeepPowerDown = 0xB9,
}

impl Opcode {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

pub struct SpiNorFlash<SPI: Transfer<u8>, CS: OutputPin> {
    spi: SPI,
    cs: CS,
}
*/
