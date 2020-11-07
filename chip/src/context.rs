use peach8::{
    embedded_graphics::{image::ImageRaw, pixelcolor::BinaryColor},
    Context,
};

pub(crate) struct DiscoveryContext;

impl Context for DiscoveryContext {
    fn on_frame<'a>(&mut self, _frame: ImageRaw<'a, BinaryColor>) {}
    fn sound_on(&mut self) {}
    fn sound_off(&mut self) {}
    fn get_keys(&mut self) -> &[bool; 16] {
        &[false; 16]
    }
    fn gen_random(&mut self) -> u8 {
        0
    }
}
