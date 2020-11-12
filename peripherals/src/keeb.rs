#![allow(unused)]
use stm32f3xx_hal as stm32f303;

use stm32f303::hal::digital::v2::{InputPin, OutputPin};

use core::convert::Infallible;

use log::info;

pub struct Keeb<'a> {
    rows: [&'a mut dyn OutputPin<Error = Infallible>; 4],
    cols: [&'a dyn InputPin<Error = Infallible>; 4],
}

impl<'a> Keeb<'a> {
    pub fn new(
        rows: [&'a mut dyn OutputPin<Error = Infallible>; 4],
        cols: [&'a dyn InputPin<Error = Infallible>; 4],
    ) -> Self {
        Self { rows, cols }
    }

    pub fn read(&mut self) -> [bool; 16] {
        let mut res = [false; 16];

        for (n, row) in self.rows.iter_mut().enumerate() {
            row.set_high().unwrap();
            for (m, col) in self.cols.iter_mut().enumerate() {
                if col.is_high().unwrap() {
                    match (n, m) {
                        (0, 0) => res[1] = true,
                        (0, 1) => res[2] = true,
                        (0, 2) => res[3] = true,
                        (0, 3) => res[0xC] = true,
                        (1, 0) => res[4] = true,
                        (1, 1) => res[5] = true,
                        (1, 2) => res[6] = true,
                        (1, 3) => res[0xD] = true,
                        (2, 0) => res[7] = true,
                        (2, 1) => res[8] = true,
                        (2, 2) => res[9] = true,
                        (2, 3) => res[0xE] = true,
                        (3, 0) => res[0xA] = true,
                        (3, 1) => res[0] = true,
                        (3, 2) => res[0xB] = true,
                        (3, 3) => res[0xF] = true,
                        (_, _) => unreachable!(),
                    }
                }
            }
            row.set_low().unwrap();
        }
        res
    }
}
