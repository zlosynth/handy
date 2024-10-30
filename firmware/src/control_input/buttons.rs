use super::debouncer::Debouncer;
use crate::system::hal::gpio;

pub const BUTTONS: usize = 2;

#[derive(Debug, defmt::Format)]
pub struct Buttons {
    buttons: [Button; BUTTONS],
    pins: Pins,
}

#[derive(Debug, defmt::Format)]
pub struct Button {
    active: bool,
    debouncer: Debouncer<4>,
}

#[derive(Debug, defmt::Format)]
pub struct Pins {
    pub button_1: Button1Pin,
    pub button_2: Button2Pin,
}

pub type Button1Pin = gpio::gpiod::PD3<gpio::Input>;
pub type Button2Pin = gpio::gpiob::PB4<gpio::Input>;

impl Buttons {
    pub fn new(pins: Pins) -> Self {
        Self {
            buttons: [Button::new(), Button::new()],
            pins,
        }
    }

    pub fn sample(&mut self) {
        self.buttons[0].set(self.pins.button_1.is_low());
        self.buttons[1].set(self.pins.button_2.is_low());
    }

    pub fn values(&self) -> [bool; BUTTONS] {
        [self.buttons[0].active, self.buttons[1].active]
    }
}

impl Button {
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
