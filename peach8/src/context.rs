use embedded_graphics::{image::ImageRaw, pixelcolor::BinaryColor};

pub trait Context {
    fn on_frame<'a>(&mut self, frame: ImageRaw<'a, BinaryColor>);
    fn on_sound(&mut self);
    fn get_keys(&mut self) -> &[bool; 16];
    fn gen_random(&mut self) -> u8;
}

#[cfg(test)]
pub mod testing {
    use super::*;

    use embedded_graphics::image::IntoPixelIter;
    use nanorand::{rand::pcg64::Pcg64 as Rng, RNG};

    use crate::utils::testing::{ImageMask, ToMask};

    pub struct TestingContext {
        sound: bool,
        frame: Option<ImageMask>,
        keys: [bool; 16],
        rng: Rng,
    }

    impl TestingContext {
        pub fn new(seed: u128) -> Self {
            Self {
                sound: false,
                frame: None,
                keys: [false; 16],
                rng: Rng::new_seed(seed),
            }
        }

        pub fn is_sound_on(&self) -> bool {
            self.sound
        }

        pub fn get_frame(&self) -> Option<&ImageMask> {
            self.frame.as_ref()
        }

        pub fn set_key(&mut self, n: u8) {
            self.keys[n as usize] = true;
        }

        pub fn reset_key(&mut self, n: u8) {
            self.keys[n as usize] = false;
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

        fn get_keys(&mut self) -> &[bool; 16] {
            &self.keys
        }
    }

    #[test]
    fn testing_context() {
        let mut ctx = TestingContext::new(0);

        let full_mask_str = include_str!("../test-data/context/full_mask");
        let full_mask_data: &[u8] = &[255; 8 * 32];
        let full_image: ImageRaw<BinaryColor> = ImageRaw::new(full_mask_data, 64, 32);

        ctx.on_frame(full_image);
        assert!(ctx.frame.is_some());
        assert_eq!(ctx.frame.unwrap(), full_mask_str.to_mask());

        ctx.on_sound();
        assert!(ctx.is_sound_on());

        ctx.set_key(0x01u8);
        ctx.set_key(0x0Fu8);
        assert_eq!(ctx.get_keys().iter().filter(|&&k| k == true).count(), 2);
        assert_eq!((ctx.keys[0x01], ctx.keys[0x0F]), (true, true));

        ctx.reset_key(0x0Fu8);
        assert_eq!(ctx.get_keys().iter().filter(|&&k| k == true).count(), 1);
        assert_eq!((ctx.keys[0x01], ctx.keys[0x0F]), (true, false));
    }
}
