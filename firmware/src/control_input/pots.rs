use nb::block;

use crate::system::hal::adc::{Adc, Enabled};
use crate::system::hal::gpio;
use crate::system::hal::pac::{ADC1, ADC2};

use super::one_pole_filter::OnePoleFilter;

pub const POTS: usize = 4;

#[derive(defmt::Format)]
pub struct Pots {
    pots: [Pot; POTS],
    pins: Pins,
}

#[derive(defmt::Format)]
pub struct Pot {
    value: f32,
    offset: f32,
    multiplier: f32,
    filter: OnePoleFilter,
}

#[derive(defmt::Format)]
pub struct Pins {
    pub pot_1: Pot1Pin,
    pub pot_2: Pot2Pin,
    pub pot_3: Pot3Pin,
    pub pot_4: Pot4Pin,
}

pub type Pot1Pin = gpio::gpioc::PC4<gpio::Analog>;
pub type Pot2Pin = gpio::gpioa::PA1<gpio::Analog>;
pub type Pot3Pin = gpio::gpiob::PB1<gpio::Analog>;
pub type Pot4Pin = gpio::gpioa::PA0<gpio::Analog>;

impl Pots {
    pub fn new(pins: Pins) -> Self {
        Self {
            pots: [
                // NOTE: To calibrate, set this to (0.0, 1.0), remove clamping
                // from Pot.set, and run diagnostics. Note the minimum and
                // maximum of each pot and then use it here.
                Pot::new(0.505, 0.99),
                Pot::new(0.99, 0.01),
                Pot::new(0.505, 0.99),
                Pot::new(0.99, 0.01),
            ],
            pins,
        }
    }

    pub fn sample(&mut self, adc_1: &mut Adc<ADC1, Enabled>, adc_2: &mut Adc<ADC2, Enabled>) {
        adc_2.start_conversion(&mut self.pins.pot_1);
        adc_1.start_conversion(&mut self.pins.pot_2);
        let sample_1: u32 = block!(adc_2.read_sample()).unwrap_or_default();
        let sample_2: u32 = block!(adc_1.read_sample()).unwrap_or_default();
        self.pots[0].set(sample_1, adc_2.slope());
        self.pots[1].set(sample_2, adc_1.slope());

        adc_2.start_conversion(&mut self.pins.pot_3);
        adc_1.start_conversion(&mut self.pins.pot_4);
        let sample_3: u32 = block!(adc_2.read_sample()).unwrap_or_default();
        let sample_4: u32 = block!(adc_1.read_sample()).unwrap_or_default();
        self.pots[2].set(sample_3, adc_2.slope());
        self.pots[3].set(sample_4, adc_1.slope());
    }

    pub fn values(&self) -> [f32; POTS] {
        [
            self.pots[0].value,
            self.pots[1].value,
            self.pots[2].value,
            self.pots[3].value,
        ]
    }
}

impl Pot {
    fn new(adc_min: f32, adc_max: f32) -> Self {
        let offset = -adc_min;
        let multiplier = 1.0 / (adc_max - adc_min);
        let filter = OnePoleFilter::new(1000.0, 10.0);
        Self {
            value: 0.0,
            offset,
            multiplier,
            filter,
        }
    }

    fn set(&mut self, sample: u32, slope: u32) {
        let phased = (slope as f32 - sample as f32) / slope as f32;
        let scaled = (phased + self.offset) * self.multiplier;
        let clamped = scaled.clamp(0.0, 1.0);
        self.value = self.filter.tick(clamped);
    }
}
