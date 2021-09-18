#![cfg_attr(not(test), no_std)]

pub mod display;
pub mod resources;

#[cfg(test)]
mod test {
    #[test]
    fn todo() {
        assert_eq!(1, 1);
    }
}
