#![no_main]
#![no_std]

#[allow(unused_imports)]
use f3;
#[cfg(not(test))]
#[allow(unused_imports)]
use panic_itm;
use cortex_m::asm::bkpt;
use cortex_m_rt::entry;

use f3::hal::stm32f30x::{self, gpioc, rcc, tim6};

use log::{info, trace, error};
use chip8_core::logger::*;

#[inline(never)]
fn delay(tim6: &tim6::RegisterBlock, ms: u16) {
    // set target value for counter
    tim6.arr.write(|w| w.arr().bits(ms));
    // enable counter
    tim6.cr1.modify(|_, w| w.cen().set_bit());
    // Wait until the alarm goes off (until the update event occurs)
    while !tim6.sr.read().uif().bit_is_set() {}
    // clear timer overflow flag
    tim6.sr.modify(|_, w| w.uif().clear_bit());
}

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take()
        .expect("Failed requesting peripherals");
    let dp = stm32f30x::Peripherals::take()
        .expect("Failed requesting peripherals");

    let logger = create_itm_logger::<InterruptOk>(LevelFilter::Trace, cp.ITM);
    unsafe { init(&logger) }

    info!("Logger Initialized");
    let (rcc, gpioc, tim6) = (dp.RCC, dp.GPIOC, dp.TIM6);
    // initialize buzzer
    info!("Powering on GPIOC");
    rcc.ahbenr.write(|w| w.iopcen().set_bit());
    info!("Setting PC10 to output mode");
    gpioc.moder.write(|w| w.moder10().output());
    // initialize timer
    // f = apb1 / ( psc + 1 )
    let psc = 7999;
    info!("Powering on Timer 6");
    rcc.apb1enr.modify(|_, w| w.tim6en().set_bit());
    tim6.cr1.write(|w| w.opm().set_bit().cen().clear_bit());
    info!("Setting on prescaler");
    tim6.psc.write(|w| w.psc().bits(psc));

    loop {
        gpioc.odr.write(|w| w.odr10().set_bit());
        delay(&tim6, 1);
        gpioc.odr.write(|w| w.odr10().clear_bit());
        delay(&tim6, 1);
    }
}
