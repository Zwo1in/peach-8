#![no_std]

use stm32f3xx_hal as stm32f303;

use stm32f303::{flash, pac, rcc, time::MegaHertz};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub mod keeb;
pub mod logger;
pub mod spu;

pub fn freeze_clocks(
    sysclk_freq: MegaHertz,
    cfgr: rcc::CFGR,
    flash: &mut flash::Parts,
) -> rcc::Clocks {
    let tpiu_async_presc = sysclk_freq.0 / 2 - 1;
    debug!("setting sysclock freq: {}mhz", sysclk_freq.0);
    trace!("update'ing tpiu's baud rate");
    trace!("setting async clock prescaller: {}", tpiu_async_presc);
    let clocks = cfgr.sysclk(sysclk_freq).freeze(&mut flash.acr);
    // tpiu is a bridge for ITM, it's asynchronous clock prescaller
    // has to be updated, otherwise logging through ITM won't work
    unsafe { (*pac::TPIU::ptr()).acpr.write(tpiu_async_presc) }
    clocks
}

#[test]
fn it_works() {
    assert_eq!(2 + 2, 4);
}
