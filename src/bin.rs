#![no_main]
#![no_std]

#[allow(unused_imports)]
use f3;
#[cfg(not(test))]
#[allow(unused_imports)]
use panic_itm;
use cortex_m::asm::bkpt;
use cortex_m_rt::entry;

use log::{info, trace, error};
use chip8_core::logger::*;

#[entry]
fn main() -> ! {
    let peripheral = cortex_m::Peripherals::take()
        .expect("Failed requesting peripherals");

    let logger = create_itm_logger::<InterruptOk>(LevelFilter::Trace, peripheral.ITM);
    unsafe { init(&logger) }

    info!("Hello {}!", "world");
    trace!("Logger Initialized");
    error!("Heap Overflow; pc:={:#x}", 0xdeadbeef_u32);

    bkpt();

    loop {}
}
