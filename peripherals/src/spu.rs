use stm32f3xx_hal as stm32f303;

use stm32f303::{
    gpio::{gpiob, AF2},
    hal::PwmPin,
    pwm::{self, PwmChannel, WithPins, TIM3_CH2},
    rcc,
    stm32::TIM3,
    time::Hertz,
};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub fn init_tim3_pwm_on_pb5(
    freq: Hertz,
    tim3: TIM3,
    pb5: gpiob::PB5<AF2>,
    clocks: &rcc::Clocks,
) -> PwmChannel<TIM3_CH2, WithPins> {
    info!("configuring timer3 in pwm mode");
    let resolution = core::u16::MAX;
    debug!("resolution: {}, frequency: {}hz", resolution, freq.0);
    debug!("using channels: 1");
    let (_, tim3_ch2, ..) = pwm::tim3(tim3, resolution, freq, &clocks);
    let mut tim3_ch2 = tim3_ch2.output_to_pb5(pb5);
    let tim3_duty = tim3_ch2.get_max_duty() / 2;
    debug!("setting timer3 duty: {}", tim3_duty);
    tim3_ch2.set_duty(tim3_duty);
    tim3_ch2
}
