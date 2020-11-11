use peach8::{
    embedded_graphics::{
        drawable::Pixel,
        geometry::Point,
        image::{ImageRaw, IntoPixelIter},
        pixelcolor::BinaryColor,
        prelude::*,
    },
    Context,
};

use ssd1306::prelude::*;

pub(crate) struct DiscoveryContext<T: WriteOnlyDataCommand> {
    pub display: GraphicsMode<T>,
}

impl<T: WriteOnlyDataCommand> Context for DiscoveryContext<T> {
    /// map image from 64x32 to 128x64
    fn on_frame<'a>(&mut self, frame: ImageRaw<'a, BinaryColor>) {
        let im_raw = frame
            .pixel_iter()
            .flat_map(|pixel| {
                (0..4).map(move |n| {
                    Pixel(
                        Point {
                            x: 2 * pixel.0.x + n % 2,
                            y: 2 * pixel.0.y + n / 2,
                        },
                        pixel.1,
                    )
                })
            })
            .for_each(|p| p.draw(&mut self.display).unwrap());
    }

    fn sound_on(&mut self) {}

    fn sound_off(&mut self) {}

    fn get_keys(&mut self) -> &[bool; 16] {
        &[false; 16]
    }

    fn gen_random(&mut self) -> u8 {
        0
    }
}
