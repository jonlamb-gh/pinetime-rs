use crate::hal::{
    gpio::{p0, Floating, Input, Output, Pin, PushPull},
    gpiote::GpioteChannel,
    prelude::OutputPin,
};

pub type ButtonEnablePin = p0::P0_15<Output<PushPull>>;
pub type ButtonPin = p0::P0_13<Input<Floating>>;

pub struct Button {
    _enable_pin: ButtonEnablePin,
    _input_pin: Pin<Input<Floating>>,
}

impl Button {
    pub fn new(
        mut enable_pin: ButtonEnablePin,
        input_pin: ButtonPin,
        channel: &GpioteChannel<'_>,
    ) -> Self {
        enable_pin.set_high().unwrap();
        let input_pin = input_pin.degrade();
        channel.input_pin(&input_pin).lo_to_hi().enable_interrupt();
        Button {
            _enable_pin: enable_pin,
            _input_pin: input_pin,
        }
    }
}
