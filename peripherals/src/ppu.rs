use stm32f3xx_hal as stm32f303;

use cortex_m::peripheral::SYST;
use ssd1306::{prelude::*, Builder};
use stm32f303::{
    delay::Delay,
    hal::digital::v2::OutputPin,
    rcc,
    spi::{MisoPin, Mode, MosiPin, Phase, Polarity, SckPin, Spi},
    stm32::SPI2,
    time::Hertz,
};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub fn init_ssd1306_on_spi2<H, SCK, MISO, MOSI, CS, DC, RST>(
    freq: H,
    spi2: SPI2,
    (sck, miso, mosi): (SCK, MISO, MOSI),
    (cs, dc, mut rst): (CS, DC, RST),
    apb1: &mut rcc::APB1,
    syst: SYST,
    clocks: rcc::Clocks,
) -> GraphicsMode<impl WriteOnlyDataCommand>
where
    H: Into<Hertz>,
    SCK: SckPin<SPI2>,
    MISO: MisoPin<SPI2>,
    MOSI: MosiPin<SPI2>,
    CS: OutputPin,
    DC: OutputPin,
    RST: OutputPin,
{
    let spi_mode = Mode {
        polarity: Polarity::IdleLow,
        phase: Phase::CaptureOnFirstTransition,
    };

    let spi = Spi::spi2(spi2, (sck, miso, mosi), spi_mode, freq.into(), clocks, apb1);

    let mut delay = Delay::new(syst, clocks);
    let interface = SPIInterface::new(spi, dc, cs);
    let mut disp: GraphicsMode<_> = Builder::new().connect(interface).into();

    let _ = disp.reset(&mut rst, &mut delay);
    disp.init().unwrap();
    info!("configuring timer3 in pwm mode");
    disp
}
