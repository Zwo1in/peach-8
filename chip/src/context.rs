use stm32f3xx_hal as stm32f303;

use stm32f303::hal::{
    timer::{CountDown, Periodic},
    PwmPin,
};

use peach8::{
    embedded_graphics::{
        drawable::{Drawable, Pixel},
        geometry::Point,
        pixelcolor::BinaryColor,
    },
    frame::FrameView,
    Context,
};

use nanorand::{rand::pcg64::Pcg64 as Rng, RNG};
use ssd1306::prelude::*;

pub(crate) struct DiscoveryContext<'a, T, U>
where
    T: WriteOnlyDataCommand,
    U: CountDown + Periodic,
{
    pub display: GraphicsMode<T>,
    pub keeb: peripherals::Keeb<'a>,
    pub buzzer: &'a mut dyn PwmPin<Duty = u16>,
    frame_timer: U,
    rng: Rng,
}

impl<'a, T, U> DiscoveryContext<'a, T, U>
where
    T: WriteOnlyDataCommand,
    U: CountDown + Periodic,
{
    pub fn new(
        display: GraphicsMode<T>,
        keeb: peripherals::Keeb<'a>,
        buzzer: &'a mut dyn PwmPin<Duty = u16>,
        frame_timer: U,
    ) -> Self {
        Self {
            display,
            keeb,
            buzzer,
            frame_timer,
            rng: Rng::new_seed(0),
        }
    }
}

impl<'a, T, U> Context for DiscoveryContext<'a, T, U>
where
    T: WriteOnlyDataCommand,
    U: CountDown + Periodic,
{
    /// map image from 64x32 to 128x64
    fn on_frame(&mut self, frame: FrameView<'_>) {
        if self.frame_timer.wait().is_ok() {
            frame
                .iter_pixelwise_scaled(2)
                .enumerate()
                .for_each(|(y, row_iter)| {
                    row_iter.enumerate().for_each(|(x, &is_on)| {
                        let p = Pixel(
                            Point::new(x as i32, y as i32),
                            if is_on {
                                BinaryColor::On
                            } else {
                                BinaryColor::Off
                            },
                        );
                        p.draw(&mut self.display).unwrap();
                    });
                });
            self.display.flush().unwrap();
        }
    }

    fn sound_on(&mut self) {
        self.buzzer.enable();
    }

    fn sound_off(&mut self) {
        self.buzzer.disable();
    }

    fn get_keys(&mut self) -> [bool; 16] {
        self.keeb.read()
    }

    fn gen_random(&mut self) -> u8 {
        self.rng.generate::<u8>()
    }
}
