use bitvec::prelude::*;

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;

pub struct Gfx([u8; WIDTH * HEIGHT / 8]);

impl Gfx {
    pub fn new() -> Self {
        Self([0; WIDTH * HEIGHT / 8])
    }

    pub fn as_raw(&self) -> &[u8] {
        &self.0
    }

    pub fn get_bit(&self, x: usize, y: usize) -> Option<&bool> {
        self.iter_rows_bitwise()
            .nth(y)
            .map(|row| row.get(x))
            .flatten()
    }

    pub fn xor_bit(&mut self, x: usize, y: usize, val: bool) -> Result<(), &'static str> {
        self.iter_rows_bitwise_mut()
            .nth(y)
            .map(|row| row.get_mut(x).map(|mut bit| *bit ^= val))
            .flatten()
            .ok_or("Pixel index out of bounds")
    }

    pub fn iter_rows_bitwise(&self) -> impl Iterator<Item = &BitSlice<Msb0, u8>> {
        self.0.chunks(WIDTH / 8).map(|row| row.view_bits::<Msb0>())
    }

    #[cfg(test)]
    pub fn as_raw_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }

    fn iter_rows_bitwise_mut(&mut self) -> impl Iterator<Item = &mut BitSlice<Msb0, u8>> {
        self.0
            .chunks_mut(WIDTH / 8)
            .map(|row| row.view_bits_mut::<Msb0>())
    }
}

#[cfg(test)]
mod gfx_test {
    use super::*;

    #[test]
    fn get_bit() {
        let mut gfx = Gfx::new();
        gfx.as_raw_mut()[0] = 0b1000_0000;

        assert_eq!(gfx.get_bit(0, 0), Some(&true),);
        assert_eq!(gfx.get_bit(1, 0), Some(&false),);
        assert_eq!(gfx.get_bit(0, 1), Some(&false),);
    }

    #[test]
    fn xor_bit() {
        let mut gfx = Gfx::new();
        gfx.xor_bit(0, 0, false).unwrap();
        assert_eq!(gfx.get_bit(0, 0), Some(&false),);
        gfx.xor_bit(0, 0, true).unwrap();
        assert_eq!(gfx.get_bit(0, 0), Some(&true),);
        gfx.xor_bit(0, 0, false).unwrap();
        assert_eq!(gfx.get_bit(0, 0), Some(&true),);
        gfx.xor_bit(0, 0, true).unwrap();
        assert_eq!(gfx.get_bit(0, 0), Some(&false),);
    }
}
