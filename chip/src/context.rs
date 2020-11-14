use stm32f3xx_hal as stm32f303;

use stm32f303::hal::{
    timer::{CountDown, Periodic},
    PwmPin,
};

use peach8::{
    embedded_graphics::{
        drawable::{Drawable, Pixel},
        geometry::Point,
        image::{ImageRaw, IntoPixelIter},
        pixelcolor::BinaryColor,
    },
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
    fn on_frame(&mut self, frame: ImageRaw<'_, BinaryColor>) {
        if self.frame_timer.wait().is_ok() {
            frame
                .pixel_iter()
                .flat_map(|Pixel(point, color)| {
                    (0..4).map(move |n| {
                        Pixel(
                            Point {
                                x: 2 * point.x + n % 2,
                                y: 2 * point.y + n / 2,
                            },
                            color,
                        )
                    })
                })
                .for_each(|p| p.draw(&mut self.display).unwrap());
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
