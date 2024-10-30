use crate::system::hal::gpio;

use super::debouncer::Debouncer;

pub const GATES: usize = 2;

#[derive(defmt::Format)]
pub struct Gates {
    triggers: [Trigger; GATES],
    pins: Pins,
}

#[derive(Debug, defmt::Format)]
pub struct Trigger {
    active: bool,
    debouncer: Debouncer<4>,
}

#[derive(defmt::Format)]
pub struct Pins {
    pub gate_1: Trigger1Pin,
    pub gate_2: Trigger2Pin,
}

pub type Trigger1Pin = gpio::gpiog::PG13<gpio::Input>;
pub type Trigger2Pin = gpio::gpiog::PG14<gpio::Input>;

impl Gates {
    pub fn new(pins: Pins) -> Self {
        Self {
            triggers: [Trigger::new(), Trigger::new()],
            pins,
        }
    }

    pub fn sample(&mut self) {
        self.triggers[0].set(self.pins.gate_1.is_high());
        self.triggers[1].set(self.pins.gate_2.is_high());
    }

    pub fn values(&self) -> [bool; GATES] {
        [self.triggers[0].active, self.triggers[1].active]
    }
}

impl Trigger {
    fn new() -> Self {
        Self {
            debouncer: Debouncer::new(),
            active: false,
        }
    }

    fn set(&mut self, is_high: bool) {
        self.active = self.debouncer.update(is_high);
    }
}
