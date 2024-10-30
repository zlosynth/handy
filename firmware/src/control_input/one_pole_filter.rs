//! Simple one-pole low-pass filter.
//!
//! Based on <https://www.earlevel.com/main/2012/12/15/a-one-pole-filter/>,
//! this filter may be useful for attribute smoothening.

use core::f32::consts::PI;

use libm::expf;

#[derive(Default, Debug, defmt::Format)]
pub struct OnePoleFilter {
    y_m1: f32,
    a0: f32,
    b1: f32,
}

impl OnePoleFilter {
    pub fn new(sample_rate: f32, cutoff: f32) -> Self {
        let normalized_frequency = cutoff / sample_rate;
        let b1 = expf(-2.0 * PI * normalized_frequency);
        let a0 = 1.0 - b1;
        Self { y_m1: 0.0, a0, b1 }
    }

    pub fn tick(&mut self, x: f32) -> f32 {
        self.y_m1 = x * self.a0 + self.y_m1 * self.b1;
        self.y_m1
    }
}
