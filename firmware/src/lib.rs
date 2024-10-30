#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_probe as _;
use stm32h7xx_hal as _;

pub mod audio;
pub mod control_input;
pub mod control_output;
pub mod queue_utils;
pub mod random_generator;
pub mod startup_sequence;
pub mod system;

// Same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked.
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
