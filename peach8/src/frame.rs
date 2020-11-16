use bitvec::prelude::*;
use embedded_graphics::{image::ImageRaw, pixelcolor::BinaryColor};

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;
pub(crate) const MEM_LENGTH: usize = WIDTH * HEIGHT / 8;

/// An opaque struct holding frame of Peach8 display
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Frame([u8; MEM_LENGTH]);

/// A shared view over a `Frame`
///
/// Has different accessors for the content of frames, which can be used independently
/// to fulfill the needs.
///
/// Each pixel is represented either by a corresponding bit being set, or by `true` value.
/// Internally, the data is stored in a form of concatenating rows from top to bottom of the frame.
/// Rows are represented as an individual bits of continuous memory, matching the state of pixels
/// from left to the right.
///
/// #Note:
/// Can return ImageRaw instance with `embedded_graphics` feature on.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct FrameView<'a>(&'a [u8; MEM_LENGTH]);

impl<'a> FrameView<'a> {
    /// View the raw memory of a frame
    pub fn as_raw(&self) -> &[u8] {
        self.0
    }

    /// Create an immutable copy of a frame
    pub fn copy_frame(self) -> Frame {
        Frame(*self.0)
    }

    /// Access frame's bits by indexes
    pub fn get_bit(&self, x: usize, y: usize) -> Option<&bool> {
        self.iter_rows_as_bitslices()
            .nth(y)
            .map(|row| row.get(x))
            .flatten()
    }

    /// Get iterator over rows in a form of a `BitSlice`s
    pub fn iter_rows_as_bitslices(&self) -> impl Iterator<Item = &'a BitSlice<Msb0, u8>> {
        self.0.chunks(WIDTH / 8).map(|row| row.view_bits::<_>())
    }

    /// Iter frame pixelwise (each pixel in row for each row in frame) after scaling it
    /// by a given factor.
    pub fn iter_pixelwise_scaled(
        &self,
        scale: usize,
    ) -> impl Iterator<Item = impl Iterator<Item = &bool>> {
        self.iter_rows_as_bitslices()
            .zip(core::iter::repeat(scale))
            .map(move |(row, scale)| {
                row.iter()
                    .flat_map(move |bit| core::iter::repeat(bit).take(scale))
            })
            .flat_map(move |row| core::iter::repeat(row).take(scale))
    }

    /// Get `ImageRaw` structure from frame's data
    #[cfg(feature = "embedded-graphics")]
    pub fn as_raw_image(&self) -> ImageRaw<'_, BinaryColor> {
        ImageRaw::new(self.as_raw(), WIDTH as u32, HEIGHT as u32)
    }
}

impl Frame {
    pub(crate) fn new() -> Self {
        Self([0; MEM_LENGTH])
    }

    /// Get view over frame
    pub fn view(&self) -> FrameView<'_> {
        FrameView(&self.0)
    }

    pub(crate) fn xor_bit(&mut self, x: usize, y: usize, val: bool) -> Result<(), &'static str> {
        self.iter_rows_as_bitslices_mut()
            .nth(y)
            .map(|row| row.get_mut(x).map(|mut bit| *bit ^= val))
            .flatten()
            .ok_or("Pixel index out of bounds")
    }

    pub(crate) fn iter_rows_as_bitslices_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut BitSlice<Msb0, u8>> {
        self.0
            .chunks_mut(WIDTH / 8)
            .map(|row| row.view_bits_mut::<_>())
    }
}

#[cfg(test)]
impl<'a> FrameView<'a> {
    pub(crate) fn new(frame: &'a [u8; MEM_LENGTH]) -> Self {
        Self(frame)
    }
}

#[cfg(test)]
impl Frame {
    pub(crate) fn as_raw_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

#[cfg(test)]
mod frame_test {
    use super::*;

    #[test]
    fn get_bit() {
        let mut frame = Frame::new();
        frame.as_raw_mut()[0] = 0b1000_0000;

        assert_eq!(frame.view().get_bit(0, 0), Some(&true));
        assert_eq!(frame.view().get_bit(1, 0), Some(&false));
        assert_eq!(frame.view().get_bit(0, 1), Some(&false));
    }

    #[test]
    fn xor_bit() {
        let mut frame = Frame::new();
        frame.xor_bit(0, 0, false).unwrap();
        assert_eq!(frame.view().get_bit(0, 0), Some(&false));
        frame.xor_bit(0, 0, true).unwrap();
        assert_eq!(frame.view().get_bit(0, 0), Some(&true));
        frame.xor_bit(0, 0, false).unwrap();
        assert_eq!(frame.view().get_bit(0, 0), Some(&true));
        frame.xor_bit(0, 0, true).unwrap();
        assert_eq!(frame.view().get_bit(0, 0), Some(&false));
    }
}
