use super::debouncer::Debouncer;
use crate::system::hal::gpio;

#[derive(Debug, defmt::Format)]
pub struct Switch {
    value: u8,
    debouncers: [Debouncer<4>; 8],
    pins: Pins,
}

#[derive(Debug, defmt::Format)]
pub struct Pins {
    pub switch_1: Switch1Pin,
    pub switch_2: Switch2Pin,
    pub switch_3: Switch3Pin,
    pub switch_4: Switch4Pin,
    pub switch_5: Switch5Pin,
    pub switch_6: Switch6Pin,
    pub switch_7: Switch7Pin,
    pub switch_8: Switch8Pin,
}

pub type Switch1Pin = gpio::gpioc::PC12<gpio::Input>;
pub type Switch2Pin = gpio::gpiod::PD2<gpio::Input>;
pub type Switch3Pin = gpio::gpioc::PC2<gpio::Input>;
pub type Switch4Pin = gpio::gpioc::PC3<gpio::Input>;
pub type Switch5Pin = gpio::gpioc::PC8<gpio::Input>;
pub type Switch6Pin = gpio::gpioc::PC9<gpio::Input>;
pub type Switch7Pin = gpio::gpioc::PC10<gpio::Input>;
pub type Switch8Pin = gpio::gpioc::PC11<gpio::Input>;

impl Switch {
    pub fn new(pins: Pins) -> Self {
        Self {
            value: 0,
            debouncers: [
                Debouncer::new(),
                Debouncer::new(),
                Debouncer::new(),
                Debouncer::new(),
                Debouncer::new(),
                Debouncer::new(),
                Debouncer::new(),
                Debouncer::new(),
            ],
            pins,
        }
    }

    // TODO: Refactor this mess
    pub fn sample(&mut self) {
        let active_1 = self.debouncers[0].update(self.pins.switch_1.is_low());
        let active_2 = self.debouncers[1].update(self.pins.switch_2.is_low());
        let active_3 = self.debouncers[2].update(self.pins.switch_3.is_low());
        let active_4 = self.debouncers[3].update(self.pins.switch_4.is_low());
        let active_5 = self.debouncers[4].update(self.pins.switch_5.is_low());
        let active_6 = self.debouncers[5].update(self.pins.switch_6.is_low());
        let active_7 = self.debouncers[6].update(self.pins.switch_7.is_low());
        let active_8 = self.debouncers[7].update(self.pins.switch_8.is_low());
        if active_1 {
            self.value = 0;
        } else if active_2 {
            self.value = 1;
        } else if active_3 {
            self.value = 2;
        } else if active_4 {
            self.value = 3;
        } else if active_5 {
            self.value = 4;
        } else if active_6 {
            self.value = 5;
        } else if active_7 {
            self.value = 6;
        } else if active_8 {
            self.value = 7;
        }
    }

    pub fn value(&self) -> u8 {
        self.value
    }
}
