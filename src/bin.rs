#![no_main]
#![no_std]

use core::cell::RefCell;

#[allow(unused_imports)]
use f3;
#[allow(unused_imports)]
use panic_itm;
use cortex_m::asm::bkpt;
use cortex_m_rt::entry;

use chip8_core::{Logger, info, trace, error};

#[entry]
fn main() -> ! {
    let peripheral = cortex_m::Peripherals::take()
        .expect("Failed requesting peripherals");

    let log = RefCell::new(Logger::new(peripheral.ITM));

    info!(log, "Hello {}!", "world");
    trace!(log, "Logger Initialized");
    error!(log, "Heap Overflow; pc:={:#x}", 0xdeadbeef_u32);

    bkpt();

    loop {}
}
