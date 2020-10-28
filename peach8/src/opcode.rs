use core::convert::TryFrom;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

/// An enum representing 36 possible opcodes of chip-8 architecture
///
/// Based on [chip8 mastering](http://mattmik.com/files/chip8/mastering/chip8.html)
///
/// Examples:
/// ```
/// use peach8::opcode::OpCode;
///
/// let instruction = 0x0ABC;
/// let opcode = OpCode::from(instruction);
///
/// assert_eq!(
///     opcode,
///     OpCode::_0NNN { nnn: 0x0ABC },
/// );
/// ```
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum OpCode {
    /// Execute machine language subroutine at address NNN
    _0NNN { nnn: u16 },
    /// Clear the screen
    _00E0,
    /// Return from a subroutine
    _00EE,
    /// Jump to address NNN
    _1NNN { nnn: u16 },
    /// Execute subroutine starting at address NNN
    _2NNN { nnn: u16 },
    /// Skip the following instruction if the value of register VX equals NN
    _3XNN { x: u8, nn: u8 },
    /// Skip the following instruction if the value of register VX is not equal to NN
    _4XNN { x: u8, nn: u8 },
    /// Skip the following instruction if the value of register VX is equal to the value of register VY
    _5XY0 { x: u8, y: u8 },
    /// Store number NN in register VX
    _6XNN { x: u8, nn: u8 },
    /// Add the value NN to register VX
    _7XNN { x: u8, nn: u8 },
    /// Store the value of register VY in register VX
    _8XY0 { x: u8, y: u8 },
    /// Set VX to VX OR VY
    _8XY1 { x: u8, y: u8 },
    /// Set VX to VX AND VY
    _8XY2 { x: u8, y: u8 },
    /// Set VX to VX XOR VY
    _8XY3 { x: u8, y: u8 },
    /// Add the value of register VY to register VX, Set VF to 01 if a carry occurs, Set VF to 00 if a carry does not occur
    _8XY4 { x: u8, y: u8 },
    /// Subtract the value of register VY from register VX, Set VF to 00 if a borrow occurs, Set VF to 01 if a borrow does not occur
    _8XY5 { x: u8, y: u8 },
    /// Store the value of register VY shifted right one bit in register VX, Set register VF to the least significant bit prior to the shift
    _8XY6 { x: u8, y: u8 },
    /// Set register VX to the value of VY minus VX, Set VF to 00 if a borrow occurs, Set VF to 01 if a borrow does not occur
    _8XY7 { x: u8, y: u8 },
    /// Store the value of register VY shifted left one bit in register VX, Set register VF to the most significant bit prior to the shift
    _8XYE { x: u8, y: u8 },
    /// Skip the following instruction if the value of register VX is not equal to the value of register VY
    _9XY0 { x: u8, y: u8 },
    /// Store memory address NNN in register I
    _ANNN { nnn: u16 },
    /// Jump to address NNN + V0
    _BNNN { nnn: u16 },
    /// Set VX to a random number with a mask of NN
    _CXNN { x: u8, nn: u8 },
    /// Draw a sprite at position VX, VY with N bytes of sprite data starting at the address stored in I, Set VF to 01 if any set pixels are changed to unset, and 00 otherwise
    _DXYN { x: u8, y: u8, n: u8 },
    /// Skip the following instruction if the key corresponding to the hex value currently stored in register VX is pressed
    _EX9E { x: u8 },
    /// Skip the following instruction if the key corresponding to the hex value currently stored in register VX is not pressed
    _EXA1 { x: u8 },
    /// Store the current value of the delay timer in register VX
    _FX07 { x: u8 },
    /// Wait for a keypress and store the result in register VX
    _FX0A { x: u8 },
    /// Set the delay timer to the value of register VX
    _FX15 { x: u8 },
    /// Set the sound timer to the value of register VX
    _FX18 { x: u8 },
    /// Add the value stored in register VX to register I
    _FX1E { x: u8 },
    /// Set I to the memory address of the sprite data corresponding to the hexadecimal digit stored in register VX
    _FX29 { x: u8 },
    /// Store the binary-coded decimal equivalent of the value stored in register VX at addresses I, I+1, and I+2
    _FX33 { x: u8 },
    /// Store the values of registers V0 to VX inclusive in memory starting at address I, I is set to I + X + 1 after operation
    _FX55 { x: u8 },
    /// Fill registers V0 to VX inclusive with the values stored in memory starting at address I, I is set to I + X + 1 after operation
    _FX65 { x: u8 },
}

impl OpCode {
    fn read_first(raw: u16) -> u8 {
        (raw >> 12 & 0x000Fu16) as u8
    }

    fn read_last(raw: u16) -> u8 {
        (raw & 0x000Fu16) as u8
    }

    fn read_x(raw: u16) -> u8 {
        (raw >> 8 & 0x000Fu16) as u8
    }

    fn read_y(raw: u16) -> u8 {
        (raw >> 4 & 0x000Fu16) as u8
    }

    fn read_nn(raw: u16) -> u8 {
        (raw & 0x00FFu16) as u8
    }

    fn read_nnn(raw: u16) -> u16 {
        raw & 0x0FFFu16
    }
}

impl TryFrom<u16> for OpCode {
    type Error = &'static str;

    fn try_from(raw: u16) -> Result<Self, Self::Error> {
        Ok(match Self::read_first(raw) {
            0x0u8 => match Self::read_nnn(raw) {
                0x0E0u16 => OpCode::_00E0,
                0x0EEu16 => OpCode::_00EE,
                nnn => OpCode::_0NNN { nnn },
            },
            0x1u8 => OpCode::_1NNN {
                nnn: Self::read_nnn(raw),
            },
            0x2u8 => OpCode::_2NNN {
                nnn: Self::read_nnn(raw),
            },
            0x3u8 => OpCode::_3XNN {
                x: Self::read_x(raw),
                nn: Self::read_nn(raw),
            },
            0x4u8 => OpCode::_4XNN {
                x: Self::read_x(raw),
                nn: Self::read_nn(raw),
            },
            0x5u8 => {
                if Self::read_last(raw) == 0u8 {
                    OpCode::_5XY0 {
                        x: Self::read_x(raw),
                        y: Self::read_y(raw),
                    }
                } else {
                    return Err("Unknown operation code");
                }
            }
            0x6u8 => OpCode::_6XNN {
                x: Self::read_x(raw),
                nn: Self::read_nn(raw),
            },
            0x7u8 => OpCode::_7XNN {
                x: Self::read_x(raw),
                nn: Self::read_nn(raw),
            },
            0x8u8 => {
                let x = Self::read_x(raw);
                let y = Self::read_y(raw);
                match Self::read_last(raw) {
                    0x0u8 => OpCode::_8XY0 { x, y },
                    0x1u8 => OpCode::_8XY1 { x, y },
                    0x2u8 => OpCode::_8XY2 { x, y },
                    0x3u8 => OpCode::_8XY3 { x, y },
                    0x4u8 => OpCode::_8XY4 { x, y },
                    0x5u8 => OpCode::_8XY5 { x, y },
                    0x6u8 => OpCode::_8XY6 { x, y },
                    0x7u8 => OpCode::_8XY7 { x, y },
                    0xEu8 => OpCode::_8XYE { x, y },
                    _ => return Err("Unknown operation code"),
                }
            }
            0x9u8 => {
                if Self::read_last(raw) == 0u8 {
                    OpCode::_9XY0 {
                        x: Self::read_x(raw),
                        y: Self::read_y(raw),
                    }
                } else {
                    return Err("Unknown operation code");
                }
            }
            0xAu8 => OpCode::_ANNN {
                nnn: Self::read_nnn(raw),
            },
            0xBu8 => OpCode::_BNNN {
                nnn: Self::read_nnn(raw),
            },
            0xCu8 => OpCode::_CXNN {
                x: Self::read_x(raw),
                nn: Self::read_nn(raw),
            },
            0xDu8 => OpCode::_DXYN {
                x: Self::read_x(raw),
                y: Self::read_y(raw),
                n: Self::read_last(raw),
            },
            0xEu8 => {
                let x = Self::read_x(raw);
                match Self::read_nn(raw) {
                    0x9Eu8 => OpCode::_EX9E { x },
                    0xA1u8 => OpCode::_EXA1 { x },
                    _ => return Err("Unknown operation code"),
                }
            }
            0xFu8 => {
                let x = Self::read_x(raw);
                match Self::read_nn(raw) {
                    0x07u8 => OpCode::_FX07 { x },
                    0x0Au8 => OpCode::_FX0A { x },
                    0x15u8 => OpCode::_FX15 { x },
                    0x18u8 => OpCode::_FX18 { x },
                    0x1Eu8 => OpCode::_FX1E { x },
                    0x29u8 => OpCode::_FX29 { x },
                    0x33u8 => OpCode::_FX33 { x },
                    0x55u8 => OpCode::_FX55 { x },
                    0x65u8 => OpCode::_FX65 { x },
                    _ => return Err("Unknown operation code"),
                }
            }
            _ => unreachable!(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_first() {
        assert_eq!(0xBu8, OpCode::read_first(0xBEEFu16));
    }

    #[test]
    fn should_read_last() {
        assert_eq!(0xFu8, OpCode::read_last(0xBEEFu16));
    }

    #[test]
    fn should_read_x() {
        assert_eq!(0xEu8, OpCode::read_x(0xDEADu16));
    }

    #[test]
    fn should_read_y() {
        assert_eq!(0xAu8, OpCode::read_y(0xDEADu16));
    }

    #[test]
    fn should_read_nn() {
        assert_eq!(0xEFu8, OpCode::read_nn(0xBEEFu16));
    }

    #[test]
    fn should_read_nnn() {
        assert_eq!(0xEEFu16, OpCode::read_nnn(0xBEEFu16));
    }

    /// OpCode may only be invalid in case where the last byte have
    /// [partially] predefined value. First byte is always valid, as
    /// whole 4 most significant bits range is used in opcodes, and the
    /// latter 4 bits are placeholders for value in each opcode
    ///
    /// Example of OpCode that may be invalid:
    ///     9AB1 (close match for OpCode::_9XY0)
    ///     ---^
    #[test]
    fn should_catch_invalid_opcodes() {
        let invalid_opcodes = [
            0x5321u16, // 5XYn
            0x843Fu16, // 8XYn
            0x8AC9u16, // 8XYn
            0x9134u16, // 9XYn
            0xE14Eu16, // EXnn
            0xE392u16, // EXnn
            0xE2B1u16, // EXnn
            0xE5A2u16, // EXnn
            0xF317u16, // FXnn
            0xF302u16, // FXnn
            0xF328u16, // FXnn
            0xF322u16, // FXnn
            0xF381u16, // FXnn
        ];
        for &raw in &invalid_opcodes {
            assert!(OpCode::try_from(raw).is_err());
        }
    }

    #[test]
    #[rustfmt::skip]
    fn should_read_all_opcodes() {
        let labeled_data = [
            (0x0ABCu16, OpCode::_0NNN { nnn: 0x0ABCu16 }),
            (0x00E0u16, OpCode::_00E0),
            (0x00EEu16, OpCode::_00EE),
            (0x1ABCu16, OpCode::_1NNN { nnn: 0x0ABCu16 }),
            (0x2ABCu16, OpCode::_2NNN { nnn: 0x0ABCu16 }),
            (0x3ABCu16, OpCode::_3XNN { x: 0xAu8, nn: 0xBCu8 }),
            (0x4ABCu16, OpCode::_4XNN { x: 0xAu8, nn: 0xBCu8 }),
            (0x5AB0u16, OpCode::_5XY0 { x: 0xAu8, y: 0xBu8 }),
            (0x6ABCu16, OpCode::_6XNN { x: 0xAu8, nn: 0xBCu8 }),
            (0x7ABCu16, OpCode::_7XNN { x: 0xAu8, nn: 0xBCu8 }),
            (0x8AB0u16, OpCode::_8XY0 { x: 0xAu8, y: 0xBu8 }),
            (0x8AB1u16, OpCode::_8XY1 { x: 0xAu8, y: 0xBu8 }),
            (0x8AB2u16, OpCode::_8XY2 { x: 0xAu8, y: 0xBu8 }),
            (0x8AB3u16, OpCode::_8XY3 { x: 0xAu8, y: 0xBu8 }),
            (0x8AB4u16, OpCode::_8XY4 { x: 0xAu8, y: 0xBu8 }),
            (0x8AB5u16, OpCode::_8XY5 { x: 0xAu8, y: 0xBu8 }),
            (0x8AB6u16, OpCode::_8XY6 { x: 0xAu8, y: 0xBu8 }),
            (0x8AB7u16, OpCode::_8XY7 { x: 0xAu8, y: 0xBu8 }),
            (0x8ABEu16, OpCode::_8XYE { x: 0xAu8, y: 0xBu8 }),
            (0x9AB0u16, OpCode::_9XY0 { x: 0xAu8, y: 0xBu8 }),
            (0xAABCu16, OpCode::_ANNN { nnn: 0x0ABCu16 }),
            (0xBABCu16, OpCode::_BNNN { nnn: 0x0ABCu16 }),
            (0xCABCu16, OpCode::_CXNN { x: 0xAu8, nn: 0xBCu8 }),
            (0xDABCu16, OpCode::_DXYN { x: 0xAu8, y: 0xBu8, n: 0xCu8 }),
            (0xEA9Eu16, OpCode::_EX9E { x: 0xAu8 }),
            (0xEAA1u16, OpCode::_EXA1 { x: 0xAu8 }),
            (0xFA07u16, OpCode::_FX07 { x: 0xAu8 }),
            (0xFA0Au16, OpCode::_FX0A { x: 0xAu8 }),
            (0xFA15u16, OpCode::_FX15 { x: 0xAu8 }),
            (0xFA18u16, OpCode::_FX18 { x: 0xAu8 }),
            (0xFA1Eu16, OpCode::_FX1E { x: 0xAu8 }),
            (0xFA29u16, OpCode::_FX29 { x: 0xAu8 }),
            (0xFA33u16, OpCode::_FX33 { x: 0xAu8 }),
            (0xFA55u16, OpCode::_FX55 { x: 0xAu8 }),
            (0xFA65u16, OpCode::_FX65 { x: 0xAu8 }),
        ];

        for &(raw, expected) in &labeled_data {
            assert_eq!(
                expected,
                OpCode::try_from(raw).unwrap(),
            );
        }
    }
}
