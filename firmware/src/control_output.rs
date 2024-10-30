use crate::system::hal::gpio;

use stm32h7xx_hal::dac::{Enabled, C1, C2};
use stm32h7xx_hal::device::DAC;
use stm32h7xx_hal::traits::DacOut;

pub struct ControlOutputState {
    pub leds: [bool; 4],
    pub gates: [bool; 2],
    pub cvs: [f32; 2],
}

pub struct ControlOutputInterface {
    pins: Pins,
    dac: (C1<DAC, Enabled>, C2<DAC, Enabled>),
}

pub struct Config {
    pub pins: Pins,
    pub dac: (C1<DAC, Enabled>, C2<DAC, Enabled>),
}

#[derive(Debug, defmt::Format)]
pub struct Pins {
    pub leds: (Led1, Led2, Led3, Led4),
    pub gates: (Gate1, Gate2),
}

type Led1 = gpio::gpiob::PB15<gpio::Output>;
type Led2 = gpio::gpiob::PB14<gpio::Output>;
type Led3 = gpio::gpiob::PB8<gpio::Output>;
type Led4 = gpio::gpiob::PB9<gpio::Output>;

type Gate1 = gpio::gpioc::PC13<gpio::Output>;
type Gate2 = gpio::gpioc::PC14<gpio::Output>;

impl ControlOutputInterface {
    pub fn new(config: Config) -> Self {
        Self {
            pins: config.pins,
            dac: config.dac,
        }
    }

    pub fn set_state(&mut self, state: &ControlOutputState) {
        self.pins.leds.0.set_state(state.leds[0].into());
        self.pins.leds.1.set_state(state.leds[1].into());
        self.pins.leds.2.set_state(state.leds[2].into());
        self.pins.leds.3.set_state(state.leds[3].into());

        self.pins.gates.0.set_state(state.gates[0].into());
        self.pins.gates.1.set_state(state.gates[1].into());

        self.dac.0.set_value(f32_cv_to_u16(state.cvs[0]));
        self.dac.1.set_value(f32_cv_to_u16(state.cvs[1]));
    }
}

fn f32_cv_to_u16(value: f32) -> u16 {
    const OUT_MIN: f32 = 0.0;
    const OUT_MAX: f32 = 5.0;
    // NOTE: The DSP works with 7 octaves, but the module can output only 5.
    // Remove the first and the last octave.
    let trimmed = (value - 1.0).clamp(0.0, 5.0);
    let desired = (trimmed - OUT_MIN) / (OUT_MAX - OUT_MIN);
    // NOTE: Measuring of DAC showed that it actually starts above 0.0 V,
    // and does not get all the way to 5.0. This compensates for that.
    let compensated = desired * (4.0 / (3.94 - 0.009)) - (0.009 / 5.0);
    let scaled = (compensated * 4096.0).clamp(0.0, 4095.999);
    scaled as u16
}
