//! Context for accessing functionalities of platform that `Peach8` is
//! emulated on.
//!
//! To ensure thread-safety execution, implementators should be `Sync`,
//! although it is not required.

use embedded_graphics::{image::ImageRaw, pixelcolor::BinaryColor};

/// Trait aggregating platform functionalities
pub trait Context {
    /// Draw current frame to the screen
    ///
    /// Called by `tick_chip` after each cycle
    fn on_frame<'a>(&mut self, frame: ImageRaw<'a, BinaryColor>);
    /// Turn sound on
    ///
    /// Called by `tick_timers` when sound timer is activated
    fn sound_on(&mut self);
    /// Turn sound off
    ///
    /// Called by `tick_timers` when sound timer is deactivated
    fn sound_off(&mut self);
    /// Get state of each key on 4x4 keyboard
    ///
    /// Called by `tick_chip` before each cycle
    fn get_keys(&mut self) -> &[bool; 16];
    /// Generate random 8-bit number
    ///
    /// Called by `tick_chip` whenever requested by executing program
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

        fn sound_on(&mut self) {
            self.sound = true;
        }

        fn sound_off(&mut self) {
            self.sound = false;
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

        ctx.sound_on();
        assert!(ctx.is_sound_on());

        ctx.sound_off();
        assert!(!ctx.is_sound_on());

        ctx.set_key(0x01u8);
        ctx.set_key(0x0Fu8);
        assert_eq!(ctx.get_keys().iter().filter(|&&k| k == true).count(), 2);
        assert_eq!((ctx.keys[0x01], ctx.keys[0x0F]), (true, true));

        ctx.reset_key(0x0Fu8);
        assert_eq!(ctx.get_keys().iter().filter(|&&k| k == true).count(), 1);
        assert_eq!((ctx.keys[0x01], ctx.keys[0x0F]), (true, false));
    }
}
