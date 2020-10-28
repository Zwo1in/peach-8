use embedded_graphics::{
    image::ImageRaw,
    pixelcolor::BinaryColor,
};

pub trait Context {
    fn on_frame<'a>(&mut self, frame: ImageRaw<'a, BinaryColor>);
    fn on_sound(&mut self);
    fn gen_random(&mut self) -> u8;
}

#[cfg(test)]
pub mod testing {
    use super::*;

    use core::fmt;

    use embedded_graphics::drawable::Pixel;
    use embedded_graphics::image::IntoPixelIter;
    use nanorand::{RNG, rand::pcg64::Pcg64 as Rng};

    #[derive(Copy, Clone, PartialEq, Eq, Hash)]
    pub struct ImageMask([[bool; 64]; 32]);

    impl ImageMask {
        fn new() -> Self {
            Self([[false; 64]; 32])
        }
    }

    impl fmt::Debug for ImageMask {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "\n").and(self.0.iter()
                .map(|&row| row
                        .iter()
                        .map(|&p| if p { write!(f, ".") } else { write!(f, " ") })
                        .fold(Ok(()), |acc, r| acc.and(r))
                        .and(write!(f, "\n")))
                .fold(Ok(()), |acc, r| acc.and(r)))
        }
    }

    pub trait ToMask {
        fn to_mask(&self) -> ImageMask;
    }

    impl ToMask for str {
        fn to_mask(&self) -> ImageMask {
            let mut mask = ImageMask::new();
            mask.0
                .iter_mut()
                .zip(self.split_whitespace())
                .for_each(|(m_row, c_row)| m_row
                    .iter_mut()
                        .zip(c_row.chars())
                        .for_each(|(m, c)| *m = if c == ' ' { false } else { true }));
            mask
        }
    }

    impl<I> ToMask for I
    where
        I: Iterator<Item = Pixel<BinaryColor>> + Clone
    {
        fn to_mask(&self) -> ImageMask {
            let mut mask = ImageMask::new();
            self.clone()
                .for_each(|Pixel(point, color)| if color == BinaryColor::On {
                    mask.0[point.y as usize][point.x as usize] = true;
                });
            mask
        }
    }

    pub struct TestingContext {
        sound: bool,
        frame: Option<ImageMask>,
        rng: Rng,
    }

    impl TestingContext {
        pub fn new(seed: u128) -> Self {
            Self {
                sound: false,
                frame: None,
                rng: Rng::new_seed(seed),
            }
        }

        pub fn is_sound_on(&self) -> bool {
            self.sound
        }

        pub fn get_frame(&self) -> Option<&ImageMask> {
            self.frame.as_ref()
        }
    }

    impl Context for TestingContext {
        fn on_frame<'a>(&mut self, frame: ImageRaw<'a, BinaryColor>) {
            self.frame = Some(frame.pixel_iter().to_mask());
        }

        fn on_sound(&mut self) {
            self.sound = true;
        }

        fn gen_random(&mut self) -> u8 {
            self.rng.generate::<u8>()
        }
    }

    #[test]
    fn to_image_mask() {
        let mask = ImageMask::new();

        let empty_mask_str = include_str!("../test-data/context/empty_mask");
        let full_mask_str = include_str!("../test-data/context/full_mask");

        let empty_mask_data: &[u8] = &[0; 8*32];
        let full_mask_data: &[u8] = &[255; 8*32];

        let empty_image: ImageRaw<BinaryColor> = ImageRaw::new(empty_mask_data, 64, 32);
        let full_image: ImageRaw<BinaryColor> = ImageRaw::new(full_mask_data, 64, 32);

        assert_eq!(
            mask,
            empty_image.pixel_iter().to_mask(),
        );

        assert_eq!(
            empty_mask_str.to_mask(),
            empty_image.pixel_iter().to_mask(),
        );

        assert_eq!(
            full_mask_str.to_mask(),
            full_image.pixel_iter().to_mask(),
        );
    }

    #[test]
    fn testing_context() {
        let mut ctx = TestingContext::new(0);

        let full_mask_str = include_str!("../test-data/context/full_mask");
        let full_mask_data: &[u8] = &[255; 8*32];
        let full_image: ImageRaw<BinaryColor> = ImageRaw::new(full_mask_data, 64, 32);

        ctx.on_frame(full_image);
        assert!(ctx.frame.is_some());
        assert_eq!(
            ctx.frame.unwrap(),
            full_mask_str.to_mask(),
        );

        ctx.on_sound();
        assert!(ctx.is_sound_on());
    }
}
