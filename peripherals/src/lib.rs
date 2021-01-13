#![no_std]

use stm32f3xx_hal as stm32f303;

use stm32f303::{pac, rcc, time::MegaHertz};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub mod keeb;
pub mod logger;
pub mod ppu;
pub mod spu;

pub use keeb::Keeb;

/// tpiu is a bridge for ITM, it's asynchronous clock prescaller
/// has to be updated, otherwise logging through ITM won't work
pub trait ClocksExt {
    fn set_tpiu_async_cpr(self, baud_rate: MegaHertz) -> Self;
}

impl ClocksExt for rcc::Clocks {
    fn set_tpiu_async_cpr(self, baud_rate: MegaHertz) -> Self {
        let tpiu_async_presc = self.hclk().0 / (baud_rate.0 * 1_000_000) - 1;
        unsafe { (*pac::TPIU::ptr()).acpr.write(tpiu_async_presc) }
        trace!("HCLK set to: {}hz", self.hclk().0);
        trace!("setting tpiu baud rate to: {}mhz", baud_rate.0);
        trace!("setting async clock prescaller: {}", tpiu_async_presc);
        self
    }
}
