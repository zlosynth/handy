mod buttons;
mod cvs;
mod debouncer;
mod gates;
mod one_pole_filter;
mod pots;
mod switch;

pub use self::buttons::Pins as ButtonsPins;
pub use self::cvs::Pins as CvsPins;
pub use self::gates::Pins as GatesPins;
pub use self::pots::Pins as PotsPins;
pub use self::switch::Pins as SwitchPins;

use self::buttons::{Buttons, BUTTONS};
use self::cvs::{Cvs, CVS};
use self::gates::{Gates, GATES};
use self::pots::{Pots, POTS};
use self::switch::Switch;
use crate::system::hal::adc::{Adc, Enabled};
use crate::system::hal::pac::{ADC1, ADC2};

pub struct ControlInputSnapshot {
    pub pots: [f32; POTS],
    pub buttons: [bool; BUTTONS],
    pub cvs: [Option<f32>; CVS],
    pub gates: [bool; GATES],
    pub switch: u8,
}

pub struct ControlInputInterface {
    pots: Pots,
    buttons: Buttons,
    cvs: Cvs,
    gates: Gates,
    switch: Switch,
    adc_1: Adc<ADC1, Enabled>,
    adc_2: Adc<ADC2, Enabled>,
}

pub struct Config {
    pub pots_pins: PotsPins,
    pub buttons_pins: ButtonsPins,
    pub cvs_pins: CvsPins,
    pub gates_pins: GatesPins,
    pub switch_pins: SwitchPins,
    pub adc_1: Adc<ADC1, Enabled>,
    pub adc_2: Adc<ADC2, Enabled>,
}

impl ControlInputInterface {
    pub fn new(config: Config) -> Self {
        Self {
            pots: Pots::new(config.pots_pins),
            buttons: Buttons::new(config.buttons_pins),
            cvs: Cvs::new(config.cvs_pins),
            gates: Gates::new(config.gates_pins),
            switch: Switch::new(config.switch_pins),
            adc_1: config.adc_1,
            adc_2: config.adc_2,
        }
    }

    pub fn sample(&mut self) {
        self.pots.sample(&mut self.adc_1, &mut self.adc_2);
        self.buttons.sample();
        self.cvs.sample(&mut self.adc_1, &mut self.adc_2);
        self.gates.sample();
        self.switch.sample();
    }

    pub fn snapshot(&self) -> ControlInputSnapshot {
        ControlInputSnapshot {
            pots: self.pots.values(),
            buttons: self.buttons.values(),
            cvs: self.cvs.values(),
            gates: self.gates.values(),
            switch: self.switch.value(),
        }
    }
}
