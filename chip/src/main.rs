#![no_main]
#![no_std]

#[allow(unused_imports)]
use panic_itm;

use cortex_m::asm::bkpt;
use cortex_m_rt::entry;

use stm32f3xx_hal as stm32f303;

use stm32f303::{
    flash::FlashExt,
    gpio::GpioExt,
    hal::PwmPin,
    pac,
    pwm,
    rcc::RccExt,
    time::U32Ext,
};

#[allow(unused_imports)]
use log::{trace, debug, info, warn, error};
use peripherals::logger::*;

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take()
        .expect("Failed requesting peripherals");
    let dp = pac::Peripherals::take()
        .expect("Failed requesting peripherals");

    let logger = create_itm_logger::<InterruptFree>(LevelFilter::Trace, cp.ITM);
    unsafe { init(&logger) }
    info!("init process started");

    debug!("configuring clocks");
    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    let sysclk_freq = 16;
    debug!("setting sysclock freq: {}mhz", sysclk_freq);
    let clocks = rcc.cfgr.sysclk(sysclk_freq.mhz()).freeze(&mut flash.acr);
    // update_tpiu_baudrate

    debug!("configuring gpiob");
    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);
    trace!("setting pb5 to alternate function 2");
    let pb5 = gpiob.pb5.into_af2(&mut gpiob.moder, &mut gpiob.afrl);

    debug!("configuring timer3 in pwm mode");
    let tim3_res = core::u16::MAX;
    let tim3_freq = 50;
    trace!("resolution: {}, frequency: {}", tim3_res, tim3_freq);
    trace!("using channels: 1");
    let (_, tim3_ch2, ..) = pwm::tim3(
        dp.TIM3,
        tim3_res,
        tim3_freq.hz(),
        &clocks,
    );
    debug!("setting timer3 channel 1 output to pb5");
    let mut tim3_ch2 = tim3_ch2.output_to_pb5(pb5);
    let tim3_duty = tim3_ch2.get_max_duty() / 2;
    trace!("setting timer3 duty: {}", tim3_duty);
    tim3_ch2.set_duty(tim3_duty);
    info!("enabling pwm from tim3 ch0 on pb5");
    tim3_ch2.enable();

    tim3_ch2.disable();

    info!("init process finished");
    loop {
    }
}
