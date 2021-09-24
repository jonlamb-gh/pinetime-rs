use crate::hal::{
    gpio::{p0, Floating, Input, Pin},
    gpiote::GpioteChannel,
    pac,
    prelude::{InputPin, _embedded_hal_adc_OneShot as OneShot},
    saadc::{self, Saadc, SaadcConfig},
};
use core::fmt;
use rtic::time::duration::Milliseconds;

/// High = battery, Low = charging.
pub type ChargeIndicationPin = p0::P0_12<Input<Floating>>;
pub type VoltagePin = p0::P0_31<Input<Floating>>;
pub type PowerPresencePin = p0::P0_19<Input<Floating>>;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(transparent)]
pub struct MilliVolts(pub u16);

impl fmt::Display for MilliVolts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} mV", self.0)
    }
}

pub struct BatteryController {
    adc: Saadc,

    charge_indication_pin: ChargeIndicationPin,
    power_presence_pin: Pin<Input<Floating>>,
    voltage_pin: VoltagePin,

    charging: bool,
    power_present: bool,
    voltage: MilliVolts,
    percent_remaining: u8,
}

impl BatteryController {
    /// Maximum voltage of battery (max charging voltage is 4.21)
    pub const BATTERY_MAX: MilliVolts = MilliVolts(4180);

    /// Minimum voltage of battery before shutdown (depends on the battery)
    pub const BATTERY_MIN: MilliVolts = MilliVolts(3200);

    pub const POWER_PRESENCE_DEBOUNCE_MS: Milliseconds<u32> = Milliseconds(200);

    pub const CHARGE_EVENT_RING_DURATION: Milliseconds<u32> = Milliseconds(30);

    pub fn new(
        adc: pac::SAADC,
        charge_indication_pin: ChargeIndicationPin,
        power_presence_pin: PowerPresencePin,
        voltage_pin: VoltagePin,
        channel: &GpioteChannel<'_>,
    ) -> Self {
        let power_presence_pin = power_presence_pin.degrade();
        channel
            .input_pin(&power_presence_pin)
            .toggle()
            .enable_interrupt();
        let adc_config = SaadcConfig {
            resolution: saadc::Resolution::_10BIT,
            oversample: saadc::Oversample::BYPASS,
            reference: saadc::Reference::INTERNAL,
            gain: saadc::Gain::GAIN1_4,
            resistor: saadc::Resistor::BYPASS,
            time: saadc::Time::_40US,
        };
        BatteryController {
            adc: Saadc::new(adc, adc_config),
            charge_indication_pin,
            power_presence_pin,
            voltage_pin,
            charging: false,
            power_present: false,
            voltage: MilliVolts(0),
            percent_remaining: 0,
        }
    }

    pub fn is_charging(&self) -> bool {
        self.charging || self.power_present
    }

    pub fn voltage(&self) -> MilliVolts {
        self.voltage
    }

    pub fn percent_remaining(&self) -> u8 {
        self.percent_remaining
    }

    pub fn update_charging_io(&mut self) -> bool {
        let mut changed = false;

        let charging = self.charge_indication_pin.is_low().unwrap();
        if charging != self.charging {
            self.charging = charging;
            // Only notify if power present changes
            //changed = true;
        }

        let power_present = self.power_presence_pin.is_low().unwrap();
        if power_present != self.power_present {
            self.power_present = power_present;
            changed = true;
        }

        changed
    }

    pub fn update_voltage(&mut self) -> bool {
        let mut changed = false;

        let voltage_raw = self
            .adc
            .read(&mut self.voltage_pin)
            .unwrap_or(0)
            .clamp(0, i16::MAX) as u32;
        let voltage = Self::raw_voltage_to_volts(voltage_raw);
        if voltage != self.voltage {
            self.voltage = voltage;
            self.percent_remaining = if voltage.0 > Self::BATTERY_MAX.0 {
                100
            } else if voltage.0 < Self::BATTERY_MIN.0 {
                0
            } else {
                let v = (voltage.0 - Self::BATTERY_MIN.0) as u32;
                let vd = (Self::BATTERY_MAX.0 - Self::BATTERY_MIN.0) as u32;
                ((v * 100) / vd) as u8
            };
            changed = true;
        }

        changed
    }

    /// Returns (ChargingStatusChanged, VoltageChanged)
    pub fn update(&mut self) -> (bool, bool) {
        let charging_changed = self.update_charging_io();
        let voltage_changed = self.update_voltage();
        (charging_changed, voltage_changed)
    }

    /// A hardware voltage divider divides the battery voltage by 2
    /// ADC gain is 1/4
    /// thus adc_voltage = battery_voltage / 2 * gain = battery_voltage / 8
    /// reference_voltage is 600mV
    /// p_event->data.done.p_buffer[0] = (adc_voltage / reference_voltage) * 1024
    fn raw_voltage_to_volts(raw: u32) -> MilliVolts {
        let mv = raw * (8 * 600) / 1024;
        MilliVolts(mv as u16)
    }
}
