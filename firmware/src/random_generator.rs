use stm32h7xx_hal::rng::{ErrorKind, Rng};

pub struct RandomGenerator {
    rng: Rng,
}

impl RandomGenerator {
    pub fn from_rng(rng: Rng) -> Self {
        Self { rng }
    }

    pub fn u16(&mut self) -> Result<u16, ErrorKind> {
        use daisy::hal::rng::RngCore;
        RngCore::<u16>::gen(&mut self.rng)
    }
}
