#![no_main]
#![no_std]

use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use daisy::pac::otg1_hs_device::diepctl2::SNAK_W;
use stm32h7xx_hal::pac::interrupt;

use handy_firmware as _;
use handy_firmware::audio::{AudioInterface, SAMPLE_RATE};
use handy_firmware::control_input::ControlInputSnapshot;
use handy_firmware::control_output::ControlOutputState;
use handy_firmware::system::System;

#[derive(Default)]
struct DualOscillator {
    phase_l: f32,
    phase_r: f32,
    progression: u32,
}

// TODO: Make it so left output plays an oscillator, right plays input, with left input half amplitude
impl DualOscillator {
    const STEP_L: f32 = 200.0 / SAMPLE_RATE as f32;
    const STEP_R: f32 = 300.0 / SAMPLE_RATE as f32;

    fn populate(&mut self, buffer: &mut [(f32, f32)]) {
        for (l, r) in buffer.iter_mut() {
            const PI_2: f32 = core::f32::consts::PI * 2.0;

            *l = libm::sinf(PI_2 * self.phase_l) * 0.5;
            self.phase_l += Self::STEP_L;
            if self.phase_l > 1.0 {
                self.phase_l -= 1.0;
            }

            *r = libm::sinf(PI_2 * self.phase_r) * 0.5;
            self.phase_r += Self::STEP_R;
            if self.phase_r > 1.0 {
                self.phase_r -= 1.0;
            }
        }

        self.progression += buffer.len() as u32;
        if self.progression < SAMPLE_RATE {
            buffer.iter_mut().for_each(|(_l, r)| *r = 0.0);
        } else if self.progression < 2 * SAMPLE_RATE {
            buffer.iter_mut().for_each(|(l, _r)| *l = 0.0);
        } else if self.progression > 3 * SAMPLE_RATE {
            self.progression = 0;
        }
    }
}

struct ControlOutputGenerator {
    index: usize,
}

impl ControlOutputGenerator {
    const LEDS: usize = 4;
    const VOCT_MIN: f32 = -5.0;
    const VOCT_MAX: f32 = 5.0;

    fn new() -> Self {
        Self { index: 0 }
    }

    // TODO: CV phase as a sine instead of rough based one LEDs
    // TODO: Test it with scope
    fn next(&mut self) -> ControlOutputState {
        let mut leds = [false; Self::LEDS];
        leds[self.index] = true;

        let cv_phase = self.index as f32 / Self::LEDS as f32;
        let cv = Self::VOCT_MIN + cv_phase * (Self::VOCT_MAX - Self::VOCT_MIN);

        self.index += 1;
        if self.index >= Self::LEDS {
            self.index -= Self::LEDS;
        }

        ControlOutputState {
            leds,
            cvs: [cv, cv * -1.0],
            gates: [self.index % 2 == 0, self.index % 4 == 0],
        }
    }
}

struct Statistics {
    pots: [PotStatistics; 4],
    input_cvs: [InputCvStatistics; 4],
    input_gates: [InputGatesStatistics; 2],
    buttons: [ButtonStatistics; 2],
    switch: SwitchStatistics,
}

impl Statistics {
    fn new() -> Self {
        Self {
            pots: [
                PotStatistics::new(),
                PotStatistics::new(),
                PotStatistics::new(),
                PotStatistics::new(),
            ],
            input_cvs: [
                InputCvStatistics::new(),
                InputCvStatistics::new(),
                InputCvStatistics::new(),
                InputCvStatistics::new(),
            ],
            input_gates: [InputGatesStatistics::new(), InputGatesStatistics::new()],
            buttons: [ButtonStatistics::new(), ButtonStatistics::new()],
            switch: SwitchStatistics::new(),
        }
    }

    fn sample(&mut self, snapshot: ControlInputSnapshot) {
        for i in 0..snapshot.pots.len() {
            self.pots[i].sample(snapshot.pots[i]);
        }
        for i in 0..snapshot.cvs.len() {
            self.input_cvs[i].sample(snapshot.cvs[i]);
        }
        for i in 0..snapshot.gates.len() {
            self.input_gates[i].sample(snapshot.gates[i]);
        }
        for i in 0..snapshot.buttons.len() {
            self.buttons[i].sample(snapshot.buttons[i]);
        }
        self.switch.sample(snapshot.switch);
    }
}

impl defmt::Format for Statistics {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "\x1B[2J\x1b[1;1H");

        defmt::write!(fmt, "Pot\tValue\t\tMin\t\tMax\t\tNoise\n");
        for (i, pot) in self.pots.iter().enumerate() {
            defmt::write!(
                fmt,
                "{}\t{}\t{}\t{}\t{}%\n",
                i + 1,
                pot.value,
                pot.min,
                pot.max,
                pot.buffer.delta() * 100.0
            );
        }

        defmt::write!(fmt, "\nI.CV\tValue\t\tMin\t\tMax\t\tNoise\n");
        for (i, cv) in self.input_cvs.iter().enumerate() {
            defmt::write!(
                fmt,
                "{}\t{}\t{}\t{}\t{}%\n",
                i + 1,
                cv.value.unwrap_or(f32::NAN),
                cv.min,
                cv.max,
                cv.buffer.delta() * 100.0 / 10.0
            );
        }

        defmt::write!(fmt, "\nI.trig\tValue\tTrig(total)\tTrig(recent)\n");
        for (i, gate) in self.input_gates.iter().enumerate() {
            defmt::write!(
                fmt,
                "{}\t{}\t{}\t\t{}\n",
                i + 1,
                gate.value,
                gate.triggered,
                gate.buffer.trues()
            );
        }

        defmt::write!(fmt, "\nButton\tValue\tTrig(total)\tTrig(recent)\n");
        for (i, button) in self.buttons.iter().enumerate() {
            defmt::write!(
                fmt,
                "{}\t{}\t{}\t\t{}\n",
                i + 1,
                button.value,
                button.triggered,
                button.buffer.trues()
            );
        }

        defmt::write!(fmt, "\nSwitch\tValue\n");
        defmt::write!(fmt, "{}\t{}\n", 1, self.switch.value,);
    }
}

struct PotStatistics {
    min: f32,
    max: f32,
    value: f32,
    buffer: F32Buffer,
}

impl PotStatistics {
    fn new() -> Self {
        Self {
            min: f32::MAX,
            max: f32::MIN,
            value: 0.0,
            buffer: F32Buffer::new(),
        }
    }

    fn sample(&mut self, value: f32) {
        self.min = f32::min(self.min, value);
        self.max = f32::max(self.max, value);
        self.value = value;
        self.buffer.write(value);
    }
}

struct InputCvStatistics {
    min: f32,
    max: f32,
    value: Option<f32>,
    buffer: F32Buffer,
}

impl InputCvStatistics {
    fn new() -> Self {
        Self {
            min: f32::MAX,
            max: f32::MIN,
            value: None,
            buffer: F32Buffer::new(),
        }
    }

    fn sample(&mut self, value: Option<f32>) {
        if let Some(value) = value {
            self.min = f32::min(self.min, value);
            self.max = f32::max(self.max, value);
            self.buffer.write(value);
        }
        self.value = value;
    }
}

struct InputGatesStatistics {
    value: bool,
    triggered: u32,
    buffer: BoolBuffer,
}

impl InputGatesStatistics {
    fn new() -> Self {
        Self {
            value: false,
            triggered: 0,
            buffer: BoolBuffer::new(),
        }
    }

    fn sample(&mut self, value: bool) {
        if !self.value && value {
            self.triggered += 1;
            self.buffer.write(true);
        } else {
            self.buffer.write(false);
        }
        self.value = value;
    }
}

struct ButtonStatistics {
    value: bool,
    triggered: u32,
    buffer: BoolBuffer,
}

impl ButtonStatistics {
    fn new() -> Self {
        Self {
            value: false,
            triggered: 0,
            buffer: BoolBuffer::new(),
        }
    }

    fn sample(&mut self, value: bool) {
        if !self.value && value {
            self.triggered += 1;
            self.buffer.write(true);
        } else {
            self.buffer.write(false);
        }
        self.value = value;
    }
}

struct SwitchStatistics {
    value: u8,
}

impl SwitchStatistics {
    fn new() -> Self {
        Self { value: 0 }
    }

    fn sample(&mut self, value: u8) {
        self.value = value;
    }
}

struct F32Buffer {
    values: [f32; 512],
    index: usize,
}

impl F32Buffer {
    fn new() -> Self {
        Self {
            values: [0.0; 512],
            index: 0,
        }
    }

    fn write(&mut self, value: f32) {
        self.values[self.index] = value;
        self.index += 1;
        if self.index >= self.values.len() {
            self.index -= self.values.len();
        }
    }

    fn delta(&self) -> f32 {
        let min: f32 = self
            .values
            .iter()
            .fold(f32::MAX, |a, b| if a < *b { a } else { *b });
        let max: f32 = self
            .values
            .iter()
            .fold(f32::MIN, |a, b| if a > *b { a } else { *b });
        max - min
    }
}

struct BoolBuffer {
    values: [bool; 512],
    index: usize,
}

impl BoolBuffer {
    fn new() -> Self {
        Self {
            values: [false; 512],
            index: 0,
        }
    }

    fn write(&mut self, value: bool) {
        self.values[self.index] = value;
        self.index += 1;
        if self.index >= self.values.len() {
            self.index -= self.values.len();
        }
    }

    fn trues(&self) -> usize {
        self.values.iter().filter(|x| **x).count()
    }
}

static AUDIO_INTERFACE: Mutex<RefCell<Option<AudioInterface>>> = Mutex::new(RefCell::new(None));
static DUAL_OSCILLATOR: Mutex<RefCell<Option<DualOscillator>>> = Mutex::new(RefCell::new(None));

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Running diagnostics");

    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = daisy::pac::Peripherals::take().unwrap();
    let system = System::init(cp, dp);

    let mut audio_interface = system.audio_interface;
    audio_interface.spawn();
    cortex_m::interrupt::free(|cs| {
        AUDIO_INTERFACE.borrow(cs).replace(Some(audio_interface));
    });
    cortex_m::interrupt::free(|cs| {
        DUAL_OSCILLATOR
            .borrow(cs)
            .replace(Some(DualOscillator::default()));
    });

    let mut statistics = Statistics::new();
    let mut control_input_interface = system.control_input_interface;

    let mut control_output_generator = ControlOutputGenerator::new();
    let mut control_output_interface = system.control_output_interface;

    // Warm up.
    for _ in 0..1000 {
        control_input_interface.sample();
        cortex_m::asm::delay(480_000);
    }

    loop {
        for _ in 0..100 {
            control_input_interface.sample();
            statistics.sample(control_input_interface.snapshot());
            cortex_m::asm::delay(1_000_000);
        }

        control_output_interface.set_state(&control_output_generator.next());
        defmt::println!("{}", statistics);
    }
}

#[interrupt]
fn DMA1_STR1() {
    cortex_m::interrupt::free(|cs| {
        if let Some(audio_interface) = AUDIO_INTERFACE.borrow(cs).borrow_mut().as_mut() {
            if let Some(dual_oscillator) = DUAL_OSCILLATOR.borrow(cs).borrow_mut().as_mut() {
                audio_interface.update_buffer(|buffer| {
                    dual_oscillator.populate(buffer);
                })
            }
        }
    });
}
