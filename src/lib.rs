#![cfg_attr(not(test), no_std)]

use nrf52832_hal as hal;

pub mod backlight;
pub mod cst816s;
pub mod display;
pub mod resources;

#[cfg(test)]
mod test {
    #[test]
    fn todo() {
        assert_eq!(1, 1);
    }
}
