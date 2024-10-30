// TODO: Simplify this by removing Option, no longer needed without probes
use nb::block;

use crate::system::hal::adc::{Adc, Enabled};
use crate::system::hal::gpio;
use crate::system::hal::pac::{ADC1, ADC2};

pub const CVS: usize = 4;

#[derive(defmt::Format)]
pub struct Cvs {
    cvs: [Cv; CVS],
    pins: Pins,
}

#[derive(Default, defmt::Format)]
pub struct Cv {
    value: Option<f32>,
}

#[derive(defmt::Format)]
pub struct Pins {
    pub cv_1: Cv1Pin,
    pub cv_2: Cv2Pin,
    pub cv_3: Cv3Pin,
    pub cv_4: Cv4Pin,
}

pub type Cv1Pin = gpio::gpioa::PA3<gpio::Analog>;
pub type Cv2Pin = gpio::gpioa::PA6<gpio::Analog>;
pub type Cv3Pin = gpio::gpioa::PA2<gpio::Analog>;
pub type Cv4Pin = gpio::gpioa::PA7<gpio::Analog>;

impl Cvs {
    pub fn new(pins: Pins) -> Self {
        Self {
            cvs: [Cv::default(), Cv::default(), Cv::default(), Cv::default()],
            pins,
        }
    }

    pub fn sample(&mut self, adc_1: &mut Adc<ADC1, Enabled>, adc_2: &mut Adc<ADC2, Enabled>) {
        adc_1.start_conversion(&mut self.pins.cv_1);
        adc_2.start_conversion(&mut self.pins.cv_2);
        let sample_1: u32 = block!(adc_1.read_sample()).unwrap_or_default();
        let sample_2: u32 = block!(adc_2.read_sample()).unwrap_or_default();
        self.cvs[0].set(sample_1, adc_1.slope());
        self.cvs[1].set(sample_2, adc_2.slope());

        adc_1.start_conversion(&mut self.pins.cv_3);
        adc_2.start_conversion(&mut self.pins.cv_4);
        let sample_3: u32 = block!(adc_1.read_sample()).unwrap_or_default();
        let sample_4: u32 = block!(adc_2.read_sample()).unwrap_or_default();
        self.cvs[2].set(sample_3, adc_1.slope());
        self.cvs[3].set(sample_4, adc_2.slope());
    }

    pub fn values(&self) -> [Option<f32>; CVS] {
        [
            self.cvs[0].value,
            self.cvs[1].value,
            self.cvs[2].value,
            self.cvs[3].value,
        ]
    }
}

impl Cv {
    fn set(&mut self, sample: u32, slope: u32) {
        let value = transpose_adc(sample, slope);
        self.value = Some(value);
    }
}

fn transpose_adc(sample: u32, slope: u32) -> f32 {
    // NOTE: The CV input theoretically spans between -5 and +5 V.
    let min = -5.0;
    let span = 10.0;

    // NOTE: Based on the measuring, most of the CV inputs actually rest at -0.02.
    let offset_compensation = 0.02;
    // NOTE: The real span of measured CV is -4.98 to +4.98 V. This compensation
    // makes sure that control value can hit both extremes.
    let scale_compensation = 10.0 / (2.0 * 4.98);

    let phase = (slope as f32 - sample as f32) / slope as f32;
    let scaled = min + phase * span;
    ((scaled + offset_compensation) * scale_compensation).clamp(min, min + span)
}
