use crate::hal::{
    gpio::{p0, Output, PushPull},
    prelude::OutputPin,
};

pub type MotorPin = p0::P0_16<Output<PushPull>>;

pub struct MotorController {
    motor_pin: MotorPin,
    is_on: bool,
}

impl MotorController {
    pub fn new(motor_pin: MotorPin) -> Self {
        let mut mc = MotorController {
            motor_pin,
            is_on: false,
        };
        mc.off();
        mc
    }

    pub fn off(&mut self) {
        self.motor_pin.set_high().unwrap();
        self.is_on = false;
    }

    pub fn on(&mut self) {
        self.motor_pin.set_low().unwrap();
        self.is_on = true;
    }

    pub fn is_on(&self) -> bool {
        self.is_on
    }
}
