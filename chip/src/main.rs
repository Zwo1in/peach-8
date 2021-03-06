#![no_main]
#![no_std]

// sets default panic handler
#[allow(unused_imports)]
use panic_itm;

#[allow(unused_imports)]
use cortex_m::asm::{bkpt, nop};
// provides _start symbol
use cortex_m_rt::entry;

use stm32f3xx_hal as stm32f303;

#[allow(unused_imports)]
use stm32f303::{
    delay::Delay,
    flash::FlashExt,
    gpio::GpioExt,
    hal::{timer::CountDown, PwmPin},
    pac,
    prelude::*,
    rcc::RccExt,
    spi::{Mode, Phase, Polarity, Spi},
    time::U32Ext,
    timer::Timer,
};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use peripherals::{logger::*, ppu, spu, ClocksExt};

use peach8::Builder;

mod context;
use context::DiscoveryContext;

#[rustfmt::skip]
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

    let hse_freq    = 8;
    let sysclk_freq = 48;
    let pclk_freq   = 24;
    let baud_rate   = 2;

    debug!("using external source with freq: {}mhz", hse_freq);
    debug!("setting sysclock freq: {}mhz", sysclk_freq);
    debug!("setting apb1 freq: {}mhz", pclk_freq);
    debug!("setting apb2 freq: {}mhz", pclk_freq);
    let clocks = rcc.cfgr
        .use_hse(hse_freq.mhz())
        .sysclk(sysclk_freq.mhz())
        .pclk1(pclk_freq.mhz())
        .pclk2(pclk_freq.mhz())
        .freeze(&mut flash.acr)
        .set_tpiu_async_cpr(baud_rate.mhz());

    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);
    let mut gpiod = dp.GPIOD.split(&mut rcc.ahb);

    info!("configuring pwm with tim3 ch1 on pb5");
    let pb5 = gpiob.pb5.into_af2(&mut gpiob.moder, &mut gpiob.afrl);
    let mut pwm_channel = spu::init_tim3_pwm_on_pb5(50.hz(), dp.TIM3, pb5, clocks);

    info!("configuring ssd1306 display via spi2");
    let rst = gpiob.pb0.into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    let dc = gpiob.pb1.into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    let cs = gpiob.pb11.into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);

    let sck = gpiob.pb13.into_af5(&mut gpiob.moder, &mut gpiob.afrh);
    let miso = gpiob.pb14.into_af5(&mut gpiob.moder, &mut gpiob.afrh);
    let mosi = gpiob.pb15.into_af5(&mut gpiob.moder, &mut gpiob.afrh);

    let spi_display = ppu::init_ssd1306_on_spi2(
        8.mhz(),
        dp.SPI2,
        (sck, miso, mosi),
        (cs, dc, rst),
        &mut rcc.apb1,
        cp.SYST,
        clocks,
    );

    info!("configuring keyboard");
    let mut pd0 = gpiod.pd0.into_push_pull_output(&mut gpiod.moder, &mut gpiod.otyper);
    let mut pd2 = gpiod.pd2.into_push_pull_output(&mut gpiod.moder, &mut gpiod.otyper);
    let mut pd4 = gpiod.pd4.into_push_pull_output(&mut gpiod.moder, &mut gpiod.otyper);
    let mut pd6 = gpiod.pd6.into_push_pull_output(&mut gpiod.moder, &mut gpiod.otyper);

    let pd1 = gpiod.pd1.into_pull_down_input(&mut gpiod.moder, &mut gpiod.pupdr);
    let pd3 = gpiod.pd3.into_pull_down_input(&mut gpiod.moder, &mut gpiod.pupdr);
    let pd5 = gpiod.pd5.into_pull_down_input(&mut gpiod.moder, &mut gpiod.pupdr);
    let pd7 = gpiod.pd7.into_pull_down_input(&mut gpiod.moder, &mut gpiod.pupdr);

    let keeb = peripherals::Keeb::new(
        [&mut pd0, &mut pd2, &mut pd4, &mut pd6],
        [&pd1, &pd3, &pd5, &pd7],
    );

    info!("setting up timers");
    let tim1_freq = 60;
    let mut tim1 = Timer::tim1(dp.TIM1, tim1_freq.hz(), clocks, &mut rcc.apb2);
    tim1.start(tim1_freq.hz());

    let tim2_freq = 500;
    let mut tim2 = Timer::tim2(dp.TIM2, tim2_freq.hz(), clocks, &mut rcc.apb1);
    tim2.start(tim2_freq.hz());

    let tim4_freq = 25;
    let mut tim4 = Timer::tim4(dp.TIM4, tim4_freq.hz(), clocks, &mut rcc.apb1);
    tim4.start(tim4_freq.hz());

    info!("setting up peach8");
    let rom = include_bytes!("../../roms/BRIX");
    let ctx = DiscoveryContext::new(spi_display, keeb, &mut pwm_channel, tim4);
    let mut chip = Builder::new()
        .with_context(ctx)
        .with_program(rom)
        .build()
        .unwrap();

    loop {
        if tim2.wait().is_ok() {
            chip.tick_chip().expect("Peach8 crashed");
        }

        if tim1.wait().is_ok() {
            chip.tick_timers();
        }
    }
}
