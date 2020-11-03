#[cfg(test)]
pub mod testing {
    use core::fmt;
    use core::ops::RangeBounds;

    use embedded_graphics::{drawable::Pixel, pixelcolor::BinaryColor};

    use crate::gfx::{Gfx, WIDTH, HEIGHT};

    #[macro_export]
    macro_rules! assert_eq_2d {
        (x_range: $xrange:expr, y_range: $yrange:expr; $lhs:expr, $rhs:expr $(,)?) => {{
            let mut lhs_mask = crate::utils::testing::ImageMask::new();
            let mut rhs_mask = crate::utils::testing::ImageMask::new();
            lhs_mask.set_slice($xrange, $yrange, &$lhs);
            rhs_mask.set_slice($xrange, $yrange, &$rhs);
            assert_eq!(lhs_mask, rhs_mask);
        }};
    }

    #[derive(Copy, Clone, PartialEq, Eq, Hash)]
    pub struct ImageMask([[bool; WIDTH]; HEIGHT]);

    impl ImageMask {
        pub fn new() -> Self {
            Self([[false; WIDTH]; HEIGHT])
        }

        pub fn offset(&mut self, xoffset: usize, yoffset: usize) -> &Self {
            let height = self.0.len();
            let width = self.0[0].len();
            for y in 0..height {
                for x in 0..width {
                    if y + yoffset < height && x + xoffset < width {
                        self.0[y + yoffset][x + xoffset] = self.0[y][x];
                        self.0[y][x] = false;
                    }
                }
            }
            self
        }

        pub fn set_slice<T>(&mut self, range_x: T, range_y: T, other: &Self)
        where
            T: RangeBounds<usize>,
        {
            let width = self.0[0].len();
            let height = self.0.len();
            for x in 0..width {
                for y in 0..height {
                    if range_x.contains(&x) && range_y.contains(&y) {
                        self.0[y][x] = other.0[y][x];
                    }
                }
            }
        }
    }

    impl fmt::Debug for ImageMask {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let width = self.0[0].len() + 2;
            write!(f, "\n")?;
            for _ in 0..width {
                write!(f, "-")?;
            }
            write!(f, "\n")?;
            for row in &self.0 {
                write!(f, "|")?;
                row.iter()
                    .map(|&p| if p { write!(f, ".") } else { write!(f, " ") })
                    .fold(Ok(()), |acc, r| acc.and(r))?;
                write!(f, "|\n")?;
            }
            for _ in 0..width {
                write!(f, "-")?;
            }
            Ok(())
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
                .for_each(|(m_row, c_row)| {
                    m_row
                        .iter_mut()
                        .zip(c_row.chars())
                        .for_each(|(m, c)| *m = if c == '#' { true } else { false })
                });
            mask
        }
    }

    impl<I> ToMask for I
    where
        I: Iterator<Item = Pixel<BinaryColor>> + Clone,
    {
        fn to_mask(&self) -> ImageMask {
            let mut mask = ImageMask::new();
            self.clone().for_each(|Pixel(point, color)| {
                if color == BinaryColor::On {
                    mask.0[point.y as usize][point.x as usize] = true;
                }
            });
            mask
        }
    }

    impl ToMask for Gfx {
        fn to_mask(&self) -> ImageMask {
            let mut mask = ImageMask::new();
            self.iter_rows_bitwise()
                .zip(mask.0.iter_mut())
                .for_each(|(g_row, m_row)| {
                    m_row
                        .iter_mut()
                        .zip(g_row)
                        .for_each(|(m, &g)| *m = g)
                });
            mask
        }
    }

    mod tests {
        use super::*;
        use embedded_graphics::{
            image::{ImageRaw, IntoPixelIter},
            pixelcolor::BinaryColor,
        };

        #[test]
        fn to_image_mask() {
            let mask = ImageMask::new();

            let empty_mask_str = include_str!("../test-data/context/empty_mask");
            let full_mask_str = include_str!("../test-data/context/full_mask");

            let empty_mask_data: &[u8] = &[0; 8 * HEIGHT];
            let full_mask_data: &[u8] = &[255; 8 * HEIGHT];

            let empty_image: ImageRaw<BinaryColor> = ImageRaw::new(empty_mask_data, WIDTH as u32, HEIGHT as u32);
            let full_image: ImageRaw<BinaryColor> = ImageRaw::new(full_mask_data, WIDTH as u32, HEIGHT as u32);

            assert_eq!(mask, empty_image.pixel_iter().to_mask());
            assert_eq!(empty_mask_str.to_mask(), empty_image.pixel_iter().to_mask());
            assert_eq!(full_mask_str.to_mask(), full_image.pixel_iter().to_mask());
        }
    }
}
