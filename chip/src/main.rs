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
use peripherals::{freeze_clocks, logger::*, ppu, spu};

use peach8::Peach8;

mod context;
use context::DiscoveryContext;

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

    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);

    info!("configuring pwm with tim3 ch1 on pb5");
    let pb5 = gpiob.pb5.into_af2(&mut gpiob.moder, &mut gpiob.afrl);
    let mut _pwm_channel = spu::init_tim3_pwm_on_pb5(50.hz(), dp.TIM3, pb5, clocks);

    info!("configuring ssd1306 display via spi2");
    let rst = gpiob
        .pb0
        .into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    let dc = gpiob
        .pb1
        .into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    let cs = gpiob
        .pb11
        .into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);

    let sck = gpiob.pb13.into_af5(&mut gpiob.moder, &mut gpiob.afrh);
    let miso = gpiob.pb14.into_af5(&mut gpiob.moder, &mut gpiob.afrh);
    let mosi = gpiob.pb15.into_af5(&mut gpiob.moder, &mut gpiob.afrh);

    let mut spi_display = ppu::init_ssd1306_on_spi2(
        8.mhz(),
        dp.SPI2,
        (sck, miso, mosi),
        (cs, dc, rst),
        &mut rcc.apb1,
        cp.SYST,
        clocks,
    );

    let tim1_freq = 60;
    let mut tim1 = Timer::tim1(dp.TIM1, tim1_freq.hz(), clocks, &mut rcc.apb2);
    tim1.start(tim1_freq.hz());

    let tim2_freq = 500;
    let mut tim2 = Timer::tim2(dp.TIM2, tim2_freq.hz(), clocks, &mut rcc.apb1);
    tim2.start(tim2_freq.hz());

    let rom = include_bytes!("../../peach8/test-data/corax89_chip8-test-rom/test_opcode.ch8");
    let ctx = DiscoveryContext {
        display: spi_display,
    };
    let mut peach8 = Peach8::load(ctx, &rom[..]);

    loop {
        if tim1.wait().is_ok() {
            info!("Tick timers!");
            peach8.tick_timers();
            peach8.ctx.display.flush().unwrap();
        }

        if tim2.wait().is_ok() {
            info!("Tick cheap!");
            peach8.tick_chip().expect("Peach8 crashed");
        }
    }
}
