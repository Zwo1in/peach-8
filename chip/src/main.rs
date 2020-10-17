#![no_main]
#![no_std]

// sets default panic handler - infinite loop
#[allow(unused_imports)]
use panic_itm;

#[allow(unused_imports)]
use cortex_m::asm::{bkpt, nop};
// provides _start symbol
use cortex_m_rt::entry;

use stm32f3xx_hal as stm32f303;

use stm32f303::{flash::FlashExt, gpio::GpioExt, hal::PwmPin, pac, rcc::RccExt, time::U32Ext};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use peripherals::{freeze_clocks, logger::*, spu};

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().expect("Failed requesting peripherals");
    let dp = pac::Peripherals::take().expect("Failed requesting peripherals");

    let logger = create_itm_logger::<InterruptFree>(LevelFilter::Trace, cp.ITM);
    unsafe { init(&logger) }
    info!("init process started");

    info!("configuring clocks");
    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    let sysclk_freq = 36.mhz();
    let clocks = freeze_clocks(sysclk_freq, rcc.cfgr, &mut flash);

    info!("configuring gpiob");
    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);
    debug!("setting pb5 to alternate function 2");
    let pb5 = gpiob.pb5.into_af2(&mut gpiob.moder, &mut gpiob.afrl);

    let mut pwm_channel = spu::init_tim3_pwm_on_pb5(50.hz(), dp.TIM3, pb5, &clocks);
    pwm_channel.enable();

    info!("init process finished");
    for _ in 0..100000 {
        nop();
    }
    info!("disabling pwm on pb5");
    pwm_channel.disable();
    loop {}
}
