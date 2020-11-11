//! Chip-8 interpreter
//!
//! Access to platform functionalities is acquired by implementing `Context` trait.
//!
//! `Peach-8` is itself thread safe if `atomic` feature is enabled (default).

use core::convert::TryInto;

use bitvec::prelude::*;
use embedded_graphics::image::ImageRaw;
use heapless::{consts::U64, Vec};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::context::Context;
use crate::gfx::{Gfx, HEIGHT, WIDTH};
use crate::opcode::OpCode;
#[cfg(feature = "atomic")]
use crate::timer::atomic::Timer;
#[cfg(not(feature = "atomic"))]
use crate::timer::racy::Timer;
use crate::timer::TimerState;

const MEM_LENGTH: usize = 4096;
const START_ADDR: u16 = 0x200;
const FONTSET_ADDR: u16 = 0x050;

/// Possible states for each key. On pressing down,
/// the key is in `Pressed` state for one cycle, and then
/// if still held, in `Down`. On releasing key, the state is
/// `Released` for one cycle, and then in `Up` state until pressed again
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum KeyState {
    Pressed,
    Down,
    Released,
    Up,
}

impl KeyState {
    #[rustfmt::skip]
    fn update(&mut self, pressed: bool) -> &Self {
        *self = match (*self, pressed) {
            (KeyState::Pressed,  true)  | (KeyState::Down, true)  => KeyState::Down,
            (KeyState::Pressed,  false) | (KeyState::Down, false) => KeyState::Released,
            (KeyState::Released, true)  | (KeyState::Up,   true)  => KeyState::Pressed,
            (KeyState::Released, false) | (KeyState::Up,   false) => KeyState::Up,
        };
        self
    }
}

/// Chip-8 virtual machine
pub struct Peach8<C: Context + Sized> {
    pub ctx: C,
    v: [u8; 16],
    i: u16,
    pc: u16,
    gfx: Gfx,
    keys: [KeyState; 16],
    stack: Vec<u16, U64>,
    memory: [u8; MEM_LENGTH],
    delay_timer: Timer,
    sound_timer: Timer,
}

impl<C: Context + Sized> Peach8<C> {
    fn new(ctx: C) -> Self {
        Self {
            ctx,
            v: [0; 16],
            i: 0,
            pc: START_ADDR,
            gfx: Gfx::new(),
            keys: [KeyState::Up; 16],
            stack: Vec::new(),
            memory: [0; MEM_LENGTH],
            delay_timer: Timer::new(),
            sound_timer: Timer::new(),
        }
    }

    /// Load program from slice of bytes to memory from 0x200 (_start address)
    pub fn load(ctx: C, prog: &[u8]) -> Self {
        let fontset: &[u8] = &[
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ];
        let mut chip = Self::new(ctx);
        chip.memory[FONTSET_ADDR as usize..]
            .iter_mut()
            .zip(fontset)
            .for_each(|(mem, &data)| *mem = data);
        chip.memory[START_ADDR as usize..]
            .iter_mut()
            .zip(prog)
            .for_each(|(mem, &data)| *mem = data);
        chip
    }

    fn pc_increment(&mut self) -> Result<(), &'static str> {
        if self.pc <= (MEM_LENGTH - 2) as u16 {
            self.pc += 2;
            Ok(())
        } else {
            Err("Attempted to increment pc out of address space")
        }
    }

    fn update_keys(&mut self) {
        self.ctx
            .get_keys()
            .iter()
            .zip(self.keys.iter_mut())
            .for_each(|(&key, state)| {
                state.update(key);
            });
    }

    fn read_opcode(&self) -> Result<OpCode, &'static str> {
        if self.pc <= (MEM_LENGTH - 2) as u16 {
            let mut opcode: u16 = 0;
            opcode |= (self.memory[self.pc as usize] as u16) << 8;
            opcode |= self.memory[(self.pc + 1) as usize] as u16;
            opcode.try_into()
        } else {
            Err("Attempted to read memory out of address space")
        }
    }

    /// Decrement delay and sound timers. Handle sound on/off events.
    ///
    /// # Note
    /// Should be called with 60Hz frequency
    pub fn tick_timers(&mut self) {
        self.delay_timer.decrement();
        match self.sound_timer.decrement() {
            TimerState::On => self.ctx.sound_on(),
            TimerState::Off => self.ctx.sound_off(),
            TimerState::Finished => (),
        }
    }

    /// Progress emulation by one cycle. Handle user input and drawing to the screen
    ///
    /// # Note
    /// Should be called with around 500Hz frequency
    pub fn tick_chip(&mut self) -> Result<(), &'static str> {
        self.update_keys();
        self.read_opcode()
            .and_then(|op| self.execute(op))
            .and({
                self.ctx.on_frame(ImageRaw::new(
                    self.gfx.as_raw(),
                    WIDTH as u32,
                    HEIGHT as u32));
                Ok(())
            })
    }
}

#[cfg(feature = "atomic")]
unsafe impl<C: Context + Sized + Sync> core::marker::Sync for Peach8<C> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::testing::TestingContext;

    #[test]
    fn pc_incrementation() {
        let mut chip = Peach8::new(TestingContext::new(0));
        assert_eq!(chip.pc, START_ADDR);
        chip.pc_increment().unwrap();
        assert_eq!(chip.pc, START_ADDR + 2);
        chip.pc_increment().unwrap();
        assert_eq!(chip.pc, START_ADDR + 4);
        chip.pc = (MEM_LENGTH - 1) as u16;
        assert_eq!(
            chip.pc_increment(),
            Err("Attempted to increment pc out of address space")
        );
    }

    #[test]
    fn key_state_update() {
        let mut state = KeyState::Pressed;
        assert_eq!(state.update(true), &KeyState::Down);
        let mut state = KeyState::Pressed;
        assert_eq!(state.update(false), &KeyState::Released);

        let mut state = KeyState::Down;
        assert_eq!(state.update(true), &KeyState::Down);
        let mut state = KeyState::Down;
        assert_eq!(state.update(false), &KeyState::Released);

        let mut state = KeyState::Released;
        assert_eq!(state.update(true), &KeyState::Pressed);
        let mut state = KeyState::Released;
        assert_eq!(state.update(false), &KeyState::Up);

        let mut state = KeyState::Up;
        assert_eq!(state.update(true), &KeyState::Pressed);
        let mut state = KeyState::Up;
        assert_eq!(state.update(false), &KeyState::Up);
    }

    #[test]
    fn update_keys() {
        let mut chip = Peach8::new(TestingContext::new(0));

        chip.ctx.set_key(0x0Fu8);
        chip.update_keys();
        assert_eq!(chip.keys[0x0Fusize], KeyState::Pressed);
        assert_eq!(chip.keys[0x02usize], KeyState::Up);
        assert_eq!(chip.keys.iter().filter(|&&k| k == KeyState::Up).count(), 15);

        chip.ctx.set_key(0x02u8);
        chip.update_keys();
        assert_eq!(chip.keys[0x0Fusize], KeyState::Down);
        assert_eq!(chip.keys[0x02usize], KeyState::Pressed);

        chip.ctx.reset_key(0x0Fu8);
        chip.update_keys();
        assert_eq!(chip.keys[0x0Fusize], KeyState::Released);
        assert_eq!(chip.keys[0x02usize], KeyState::Down);

        chip.ctx.reset_key(0x02u8);
        chip.update_keys();
        assert_eq!(chip.keys[0x0Fusize], KeyState::Up);
        assert_eq!(chip.keys[0x02usize], KeyState::Released);
    }

    #[test]
    fn timers_tick() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        chip.assign_vx_nn(0, 101)?;
        chip.assign_delay_t_vx(0)?;
        chip.assign_sound_t_vx(0)?;
        assert!(!chip.ctx.is_sound_on());

        chip.tick_timers();
        assert!(chip.ctx.is_sound_on());

        for _ in 0..100 {
            chip.tick_timers();
        }
        assert!(chip.ctx.is_sound_on());
        assert_eq!(chip.delay_timer.load(), 0);
        assert_eq!(chip.sound_timer.load(), 0);

        chip.tick_timers();
        assert!(!chip.ctx.is_sound_on());

        Ok(())
    }

    #[test]
    fn read_opcode() -> Result<(), &'static str> {
        let mut chip = Peach8::load(TestingContext::new(0), &[0x14u8, 0x65u8]);
        let opcode = chip.read_opcode()?;
        assert_eq!(opcode, OpCode::_1NNN { nnn: 0x465u16 },);

        chip.pc = (MEM_LENGTH - 1) as u16;
        assert_eq!(
            chip.read_opcode(),
            Err("Attempted to read memory out of address space"),
        );
        Ok(())
    }
}

// OpCodes impls
impl<C: Context + Sized> Peach8<C> {
    #[rustfmt::skip]
    fn execute(&mut self, opcode: OpCode) -> Result<(), &'static str>{
        match opcode {
            OpCode::_0NNN { nnn }     => return self.exec_ml_subroutine_at(nnn),
            OpCode::_00E0             => self.clear_screen(),
            OpCode::_00EE             => self.subroutine_return(),
            OpCode::_1NNN { nnn }     => return self.jump_to(nnn),
            OpCode::_2NNN { nnn }     => return self.exec_subroutine_at(nnn),
            OpCode::_3XNN { x, nn }   => self.skip_if_vx_eq_nn(x, nn),
            OpCode::_4XNN { x, nn }   => self.skip_if_vx_ne_nn(x, nn),
            OpCode::_5XY0 { x, y }    => self.skip_if_vx_eq_vy(x, y),
            OpCode::_6XNN { x, nn }   => self.assign_vx_nn(x, nn),
            OpCode::_7XNN { x, nn }   => self.assign_add_vx_nn(x, nn),
            OpCode::_8XY0 { x, y }    => self.assign_vx_vy(x, y),
            OpCode::_8XY1 { x, y }    => self.assign_or_vx_vy(x, y),
            OpCode::_8XY2 { x, y }    => self.assign_and_vx_vy(x, y),
            OpCode::_8XY3 { x, y }    => self.assign_xor_vx_vy(x, y),
            OpCode::_8XY4 { x, y }    => self.assign_add_vx_vy(x, y),
            OpCode::_8XY5 { x, y }    => self.assign_sub_vx_vy(x, y),
            OpCode::_8XY6 { x, y }    => self.assign_vx_vy_shifted_r(x, y),
            OpCode::_8XY7 { x, y }    => self.assign_vx_vy_sub_vx(x, y),
            OpCode::_8XYE { x, y }    => self.assign_vx_vy_shifted_l(x, y),
            OpCode::_9XY0 { x, y }    => self.skip_if_vx_ne_vy(x, y),
            OpCode::_ANNN { nnn }     => self.assign_i_nnn(nnn),
            OpCode::_BNNN { nnn }     => return self.jump_to_nnn_add_v0(nnn),
            OpCode::_CXNN { x, nn }   => self.assign_vx_ranom_and_nn(x, nn),
            OpCode::_DXYN { x, y, n } => self.draw_n_at_vx_vy(x, y, n),
            OpCode::_EX9E { x }       => self.skip_if_vx_in_keys(x),
            OpCode::_EXA1 { x }       => self.skip_if_vx_not_in_keys(x),
            OpCode::_FX07 { x }       => self.assign_vx_delay_t(x),
            OpCode::_FX0A { x }       => return self.assign_vx_wait_for_key(x),
            OpCode::_FX15 { x }       => self.assign_delay_t_vx(x),
            OpCode::_FX18 { x }       => self.assign_sound_t_vx(x),
            OpCode::_FX1E { x }       => self.assign_add_i_vx(x),
            OpCode::_FX29 { x }       => self.assign_i_addr_of_sprite_vx(x),
            OpCode::_FX33 { x }       => self.assign_mem_at_i_bcd_of_vx(x),
            OpCode::_FX55 { x }       => self.assign_mem_at_i_v0_to_vx(x),
            OpCode::_FX65 { x }       => self.assign_v0_to_vx_mem_at_i(x),
        }
        .and(self.pc_increment())
    }

    /// Execute machine language subroutine at address NNN
    /// 0NNN { nnn: u16 },
    fn exec_ml_subroutine_at(&mut self, _nnn: u16) -> Result<(), &'static str> {
        Err("Machine code subroutines not supported")
    }

    /// Clear the screen
    /// 00E0,
    fn clear_screen(&mut self) -> Result<(), &'static str> {
        self.gfx = Gfx::new();
        Ok(())
    }

    /// Return from a subroutine
    /// 00EE,
    fn subroutine_return(&mut self) -> Result<(), &'static str> {
        self.stack
            .pop()
            .ok_or("Can't return. Not in subroutine")
            .map(|addr| self.pc = addr)
    }

    /// Jump to address NNN
    /// 1NNN { nnn: u16 },
    fn jump_to(&mut self, nnn: u16) -> Result<(), &'static str> {
        if nnn < START_ADDR {
            Err("Attempted to jump out of program's address space")
        } else {
            self.pc = nnn;
            Ok(())
        }
    }

    /// Execute subroutine starting at address NNN
    /// 2NNN { nnn: u16 },
    fn exec_subroutine_at(&mut self, nnn: u16) -> Result<(), &'static str> {
        if nnn < START_ADDR {
            Err("Attempted to jump out of program's address space")
        } else {
            self.stack
                .push(self.pc)
                .or(Err("Cannot enter subroutine, stack is full"))
                .map(|_| self.pc = nnn)
        }
    }

    /// Skip the following instruction if the value of register VX equals NN
    /// 3XNN { x: u8, nn: u8 },
    fn skip_if_vx_eq_nn(&mut self, x: u8, nn: u8) -> Result<(), &'static str> {
        if self.v[x as usize] == nn {
            self.pc_increment()
        } else {
            Ok(())
        }
    }

    /// Skip the following instruction if the value of register VX is not equal to NN
    /// 4XNN { x: u8, nn: u8 },
    fn skip_if_vx_ne_nn(&mut self, x: u8, nn: u8) -> Result<(), &'static str> {
        if self.v[x as usize] != nn {
            self.pc_increment()
        } else {
            Ok(())
        }
    }

    /// Skip the following instruction if the value of register VX is equal to the value of register VY
    /// 5XY0 { x: u8, y: u8 },
    fn skip_if_vx_eq_vy(&mut self, x: u8, y: u8) -> Result<(), &'static str> {
        if self.v[x as usize] == self.v[y as usize] {
            self.pc_increment()
        } else {
            Ok(())
        }
    }

    /// Store number NN in register VX
    /// 6XNN { x: u8, nn: u8 },
    fn assign_vx_nn(&mut self, x: u8, nn: u8) -> Result<(), &'static str> {
        self.v[x as usize] = nn;
        Ok(())
    }

    /// Add the value NN to register VX
    /// 7XNN { x: u8, nn: u8 },
    fn assign_add_vx_nn(&mut self, x: u8, nn: u8) -> Result<(), &'static str> {
        self.v[x as usize] = self.v[x as usize].wrapping_add(nn);
        Ok(())
    }

    /// Store the value of register VY in register VX
    /// 8XY0 { x: u8, y: u8 },
    fn assign_vx_vy(&mut self, x: u8, y: u8) -> Result<(), &'static str> {
        self.v[x as usize] = self.v[y as usize];
        Ok(())
    }

    /// Set VX to VX OR VY
    /// 8XY1 { x: u8, y: u8 },
    fn assign_or_vx_vy(&mut self, x: u8, y: u8) -> Result<(), &'static str> {
        self.v[x as usize] |= self.v[y as usize];
        Ok(())
    }

    /// Set VX to VX AND VY
    /// 8XY2 { x: u8, y: u8 },
    fn assign_and_vx_vy(&mut self, x: u8, y: u8) -> Result<(), &'static str> {
        self.v[x as usize] &= self.v[y as usize];
        Ok(())
    }

    /// Set VX to VX XOR VY
    /// 8XY3 { x: u8, y: u8 },
    fn assign_xor_vx_vy(&mut self, x: u8, y: u8) -> Result<(), &'static str> {
        self.v[x as usize] ^= self.v[y as usize];
        Ok(())
    }

    /// Add the value of register VY to register VX, Set VF to 01 if a carry occurs, Set VF to 00 if a carry does not occur
    /// 8XY4 { x: u8, y: u8 },
    fn assign_add_vx_vy(&mut self, x: u8, y: u8) -> Result<(), &'static str> {
        let (value, overflow) = self.v[x as usize].overflowing_add(self.v[y as usize]);
        self.v[x as usize] = value;
        self.v[15] = if !overflow { 0x00u8 } else { 0x01u8 };
        Ok(())
    }

    /// Subtract the value of register VY from register VX, Set VF to 00 if a borrow occurs, Set VF to 01 if a borrow does not occur
    /// 8XY5 { x: u8, y: u8 },
    fn assign_sub_vx_vy(&mut self, x: u8, y: u8) -> Result<(), &'static str> {
        let (value, borrow) = self.v[x as usize].overflowing_sub(self.v[y as usize]);
        self.v[x as usize] = value;
        self.v[15] = if borrow { 0x00u8 } else { 0x01u8 };
        Ok(())
    }

    /// Store the value of register VY shifted right one bit in register VX, Set register VF to the least significant bit prior to the shift
    /// 8XY6 { x: u8, y: u8 },
    fn assign_vx_vy_shifted_r(&mut self, x: u8, y: u8) -> Result<(), &'static str> {
        let lsb = self.v[y as usize] & 1u8;
        let value = self.v[y as usize].wrapping_shr(1);
        self.v[x as usize] = value;
        self.v[y as usize] = value;
        self.v[15] = lsb;
        Ok(())
    }

    /// Set register VX to the value of VY minus VX, Set VF to 00 if a borrow occurs, Set VF to 01 if a borrow does not occur
    /// 8XY7 { x: u8, y: u8 },
    fn assign_vx_vy_sub_vx(&mut self, x: u8, y: u8) -> Result<(), &'static str> {
        let (value, borrow) = self.v[y as usize].overflowing_sub(self.v[x as usize]);
        self.v[x as usize] = value;
        self.v[15] = if borrow { 0x00u8 } else { 0x01u8 };
        Ok(())
    }

    /// Store the value of register VY shifted left one bit in register VX, Set register VF to the most significant bit prior to the shift
    /// 8XYE { x: u8, y: u8 },
    fn assign_vx_vy_shifted_l(&mut self, x: u8, y: u8) -> Result<(), &'static str> {
        let msb = self.v[y as usize] >> 7;
        let value = self.v[y as usize].wrapping_shl(1);
        self.v[x as usize] = value;
        self.v[y as usize] = value;
        self.v[15] = msb;
        Ok(())
    }

    /// Skip the following instruction if the value of register VX is not equal to the value of register VY
    /// 9XY0 { x: u8, y: u8 },
    fn skip_if_vx_ne_vy(&mut self, x: u8, y: u8) -> Result<(), &'static str> {
        if self.v[x as usize] != self.v[y as usize] {
            self.pc_increment()
        } else {
            Ok(())
        }
    }

    /// Store memory address NNN in register I
    /// ANNN { nnn: u16 },
    fn assign_i_nnn(&mut self, nnn: u16) -> Result<(), &'static str> {
        self.i = nnn;
        Ok(())
    }

    /// Jump to address NNN + V0
    /// BNNN { nnn: u16 },
    fn jump_to_nnn_add_v0(&mut self, nnn: u16) -> Result<(), &'static str> {
        let addr = nnn + self.v[0] as u16;
        if addr < START_ADDR {
            Err("Attempted to jump out of program's address space")
        } else if addr < MEM_LENGTH as u16 {
            self.pc = addr;
            Ok(())
        } else {
            Err("Attempted to set pc out of address space")
        }
    }

    /// Set VX to a random number with a mask of NN
    /// CXNN { x: u8, nn: u8 },
    fn assign_vx_ranom_and_nn(&mut self, x: u8, nn: u8) -> Result<(), &'static str> {
        let value = self.ctx.gen_random() & nn;
        self.v[x as usize] = value;
        Ok(())
    }

    /// Draw a sprite at position VX, VY with N bytes of sprite data starting at the address stored in I, Set VF to 01 if any set pixels are changed to unset, and 00 otherwise
    /// DXYN { x: u8, y: u8, n: u8 },
    fn draw_n_at_vx_vy(&mut self, x: u8, y: u8, n: u8) -> Result<(), &'static str> {
        if self.i + n as u16 >= MEM_LENGTH as u16 {
            return Err("Attempted to read memory out of address space");
        }

        let x = self.v[x as usize] as usize % WIDTH;
        let y = self.v[y as usize] as usize % HEIGHT;
        let x_stop = core::cmp::min(x + 8 as usize, WIDTH);
        let y_stop = core::cmp::min(y + n as usize, HEIGHT);

        let mut collision = false;
        for x_idx in x..x_stop {
            for y_idx in y..y_stop {
                let row =
                    BitSlice::<Msb0, _>::from_element(&self.memory[self.i as usize + y_idx - y]);
                let to_draw = *row.get(x_idx - x).unwrap();
                let curr_bit = *self.gfx.get_bit(x_idx, y_idx).unwrap();
                if to_draw && to_draw == curr_bit {
                    collision = true;
                }
                self.gfx.xor_bit(x_idx, y_idx, to_draw)?;
            }
        }

        self.v[15] = if collision { 0x01u8 } else { 0x00u8 };
        Ok(())
    }

    /// Skip the following instruction if the key corresponding to the hex value currently stored in register VX is pressed
    /// EX9E { x: u8 },
    fn skip_if_vx_in_keys(&mut self, x: u8) -> Result<(), &'static str> {
        let key = self.v[x as usize];
        if key < 0x10u8 && [KeyState::Pressed, KeyState::Down].contains(&self.keys[key as usize]) {
            return self.pc_increment();
        }
        Ok(())
    }

    /// Skip the following instruction if the key corresponding to the hex value currently stored in register VX is not pressed
    /// EXA1 { x: u8 },
    fn skip_if_vx_not_in_keys(&mut self, x: u8) -> Result<(), &'static str> {
        let key = self.v[x as usize];
        if key < 0x10u8 && [KeyState::Pressed, KeyState::Down].contains(&self.keys[key as usize]) {
            return Ok(());
        }
        self.pc_increment()
    }

    /// Store the current value of the delay timer in register VX
    /// FX07 { x: u8 },
    fn assign_vx_delay_t(&mut self, x: u8) -> Result<(), &'static str> {
        self.v[x as usize] = self.delay_timer.load();
        Ok(())
    }

    /// Wait for a keypress and store the result in register VX
    /// FX0A { x: u8 },
    fn assign_vx_wait_for_key(&mut self, x: u8) -> Result<(), &'static str> {
        let key = self.keys.iter().enumerate().find_map(|(n, &key)| {
            if key == KeyState::Released {
                Some(n)
            } else {
                None
            }
        });

        if let Some(key) = key {
            self.v[x as usize] = key as u8;
            self.pc_increment()
        } else {
            Ok(())
        }
    }

    /// Set the delay timer to the value of register VX
    /// FX15 { x: u8 },
    fn assign_delay_t_vx(&mut self, x: u8) -> Result<(), &'static str> {
        self.delay_timer.store(self.v[x as usize]);
        Ok(())
    }

    /// Set the sound timer to the value of register VX
    /// FX18 { x: u8 },
    fn assign_sound_t_vx(&mut self, x: u8) -> Result<(), &'static str> {
        self.sound_timer.store(self.v[x as usize]);
        Ok(())
    }

    /// Add the value stored in register VX to register I
    /// FX1E { x: u8 },
    fn assign_add_i_vx(&mut self, x: u8) -> Result<(), &'static str> {
        let addr = self.i + self.v[x as usize] as u16;
        if addr < MEM_LENGTH as u16 {
            self.i = addr;
            Ok(())
        } else {
            Err("Attempted to set i out of address space")
        }
    }

    /// Set I to the memory address of the sprite data corresponding to the hexadecimal digit stored in register VX
    /// FX29 { x: u8 },
    fn assign_i_addr_of_sprite_vx(&mut self, x: u8) -> Result<(), &'static str> {
        let value = (self.v[x as usize] % 16) as u16;
        self.i = FONTSET_ADDR + value * 5;
        Ok(())
    }

    /// Store the binary-coded decimal equivalent of the value stored in register VX at addresses I, I+1, and I+2
    /// FX33 { x: u8 },
    fn assign_mem_at_i_bcd_of_vx(&mut self, x: u8) -> Result<(), &'static str> {
        if ((self.i + 2) as usize) < self.memory.len() {
            let value = self.v[x as usize];
            self.memory[self.i as usize] = value / 100u8;
            self.memory[(self.i + 1) as usize] = (value % 100) / 10u8;
            self.memory[(self.i + 2) as usize] = value % 10u8;
            Ok(())
        } else {
            Err("Attempted to set memory out of address space")
        }
    }

    /// Store the values of registers V0 to VX inclusive in memory starting at address I, I is set to I + X + 1 after operation
    /// FX55 { x: u8 },
    fn assign_mem_at_i_v0_to_vx(&mut self, x: u8) -> Result<(), &'static str> {
        if ((self.i + x as u16) as usize) < self.memory.len() - 1 {
            for idx in 0..=x {
                self.memory[self.i as usize] = self.v[idx as usize];
                self.i += 1
            }
            Ok(())
        } else {
            Err("Attempted to store data out of address space")
        }
    }

    /// Fill registers V0 to VX inclusive with the values stored in memory starting at address I, I is set to I + X + 1 after operation
    /// FX65 { x: u8 },
    fn assign_v0_to_vx_mem_at_i(&mut self, x: u8) -> Result<(), &'static str> {
        if ((self.i + x as u16) as usize) < self.memory.len() - 1 {
            for idx in 0..=x {
                self.v[idx as usize] = self.memory[self.i as usize];
                self.i += 1
            }
            Ok(())
        } else {
            Err("Attempted to load memory out of address space")
        }
    }
}

#[cfg(test)]
mod opcodes_execution_tests {
    use super::*;

    use core::convert::{TryFrom, TryInto};

    use crate::assert_eq_2d;
    use crate::context::testing::TestingContext;
    use crate::utils::testing::ToMask;

    #[test]
    fn pc_manipulation_test() -> Result<(), &'static str> {
        let no_jump_opcodes = [
            0x00E0u16, // 00E0 clear_screen()
            0x6BAAu16, // 6XNN assign_vx_nn(x nn)
            0x7BAAu16, // 7XNN assign_add_vx_nn(x nn)
            0x8BC0u16, // 8XY0 assign_vx_vy(x y)
            0x8BC1u16, // 8XY1 assign_or_vx_vy(x y)
            0x8BC2u16, // 8XY2 assign_and_vx_vy(x y)
            0x8BC3u16, // 8XY3 assign_xor_vx_vy(x y)
            0x8BC4u16, // 8XY4 assign_add_vx_vy(x y)
            0x8BC5u16, // 8XY5 assign_sub_vx_vy(x y)
            0x8BC6u16, // 8XY6 assign_vx_vy_shifted_r(x y)
            0x8BC7u16, // 8XY7 assign_vx_vy_sub_vx(x y)
            0x8BCEu16, // 8XYE assign_vx_vy_shifted_l(x y)
            0xAAAAu16, // ANNN assign_i_nnn(nnn)
            0xCBAAu16, // CXNN assign_vx_ranom_and_nn(x nn)
            0xDBCAu16, // DXYN draw_n_at_vx_vy(x y n)
            0xFB07u16, // FX07 assign_vx_delay_t(x)
            0xFB15u16, // FX15 assign_delay_t_vx(x)
            0xFB18u16, // FX18 assign_sound_t_vx(x)
            0xFB1Eu16, // FX1E assign_add_i_vx(x)
            0xFB29u16, // FX29 assign_i_addr_of_sprite_vx(x)
            0xFB33u16, // FX33 assign_mem_at_i_bcd_of_vx(x)
            0xFB55u16, // FX55 assign_mem_at_i_v0_to_vx(x)
            0xFB65u16, // FX65 assign_v0_to_vx_mem_at_i(x)
        ];
        // Let the skip never be present, it is validated somewhere else
        let skip_opcodes = [
            0x3BAAu16, // 3XNN skip_if_vx_eq_nn(x nn)
            0x4B00u16, // 4XNN skip_if_vx_ne_nn(x nn)
            0x5BC0u16, // 5XY0 skip_if_vx_eq_vy(x y)
            0x9BB0u16, // 9XY0 skip_if_vx_ne_vy(x y)
            0xEB9Eu16, // EX9E skip_if_vx_in_keys(x)
            0xECA1u16, // EXA1 skip_if_vx_not_in_keys(x)
        ];
        let jump_opcodes = [
            0x1AAAu16, // 1NNN jump_to(nnn)
            0x2AAAu16, // 2NNN exec_subroutine_at(nnn)
            0xBAAAu16, // BNNN jump_to_nnn_add_v0(nnn)
        ];
        // Should increment pc, but nevertheless is a jump
        let return_subr_opcode = 0x00EEu16; // 00EE subroutine_return()
                                            // Wait for a keypress should stop execution, hovewer to not block the whole routine
                                            // it just doesn't increment pc until wait condition is met
        let wait_opcode = 0xFB0Au16; // FX0A assign_vx_wait_for_key(x)
                                     // This always returns Err
        let _ommited = 0x0AAAu16; // 0NNN exec_ml_subroutine_at(nnn)

        let mut chip = Peach8::new(TestingContext::new(0));
        let mut pc = chip.pc;

        for op in &no_jump_opcodes {
            chip.execute((*op).try_into()?)?;
            assert_eq!(chip.pc, pc + 2);
            pc += 2;
        }

        chip.assign_vx_nn(0xC, 0x01u8)?; // no skip 5XY0
        chip.ctx.set_key(0x01u8); // no skip EXA1
        chip.update_keys();
        for op in &skip_opcodes {
            chip.execute((*op).try_into()?)?;
            assert_eq!(chip.pc, pc + 2);
            pc += 2;
        }

        for op in &jump_opcodes {
            chip.execute((*op).try_into()?)?;
            assert_eq!(chip.pc, 0x0AAAu16);
        }

        chip.pc = START_ADDR;
        chip.exec_subroutine_at(0x0BBB)?;
        chip.execute(return_subr_opcode.try_into()?)?;
        assert_eq!(chip.pc, 0x202);

        chip.execute(wait_opcode.try_into()?)?;
        assert_eq!(chip.pc, 0x202);
        chip.ctx.set_key(0x01);
        chip.update_keys();
        chip.ctx.reset_key(0x01);
        chip.update_keys();
        chip.execute(wait_opcode.try_into()?)?;
        assert_eq!(chip.pc, 0x204);

        Ok(())
    }

    /// Execute machine language subroutine at address NNN
    #[test]
    fn execute_0nnn_exec_ml_subroutine_at() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::try_from(0x0000u16)?;
        assert_eq!(
            chip.execute(opcode),
            Err("Machine code subroutines not supported"),
        );
        Ok(())
    }

    /// Clear the screen
    #[test]
    fn execute_00e0_clear_screen() -> Result<(), &'static str> {
        let mut chip = Peach8::load(TestingContext::new(0), &[]);
        let opcode = OpCode::_00E0;
        let empty_mask_str = include_str!("../test-data/context/empty_mask");

        assert_eq_2d!(
            x_range: .., y_range: ..;
            chip.gfx.to_mask(), empty_mask_str.to_mask()
        );

        chip.assign_vx_nn(1, 0x0F)?;
        chip.assign_i_addr_of_sprite_vx(1)?;
        chip.draw_n_at_vx_vy(0, 0, 5)?;
        assert_eq_2d!(
            x_range: 0..8, y_range: 0..5;
            chip.gfx.to_mask(), "####....
                                 #.......
                                 ####....
                                 #.......
                                 #.......".to_mask()
        );

        chip.execute(opcode)?;
        assert_eq_2d!(
            x_range: .., y_range: ..;
            chip.gfx.to_mask(), empty_mask_str.to_mask()
        );
        Ok(())
    }

    /// Return from a subroutine
    #[test]
    fn execute_00ee_subroutine_return() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::try_from(0x00EEu16)?;
        let jumps = [0x260u16, 0x7F1u16, 0xFA2u16, 0x333u16];
        jumps
            .iter()
            .map(|&addr| OpCode::_2NNN { nnn: addr })
            .for_each(|op| chip.execute(op).unwrap());
        assert_eq!(chip.pc, 0x333u16);

        for &addr in jumps.iter().rev().skip(1) {
            chip.execute(opcode)?;
            assert_eq!(chip.pc, addr + 2u16); // +2 because pc increments on return
        }
        chip.execute(opcode)?;
        assert_eq!(chip.pc, 0x202u16);

        assert_eq!(chip.execute(opcode), Err("Can't return. Not in subroutine"));
        Ok(())
    }

    /// Jump to address NNN
    #[test]
    fn execute_1nnn_jump_to() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::try_from(0x1220u16)?;
        chip.execute(opcode)?;
        assert_eq!(chip.pc, 0x220u16);
        let opcode = OpCode::try_from(0x1FFFu16)?;
        chip.execute(opcode)?;
        assert_eq!(chip.pc, MEM_LENGTH as u16 - 1);
        let opcode = OpCode::try_from(0x1000u16)?;
        assert_eq!(
            chip.execute(opcode),
            Err("Attempted to jump out of program's address space"),
        );
        Ok(())
    }

    /// Execute subroutine starting at address NNN
    #[test]
    fn execute_2nnn_exec_subroutine_at() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let subr_addr = 0x222u16;
        let opcode = OpCode::_2NNN { nnn: subr_addr };
        chip.execute(opcode)?;
        assert_eq!(chip.pc, subr_addr);
        assert_eq!(chip.stack.len(), 1);
        assert_eq!(chip.stack.pop().unwrap(), START_ADDR);

        for _ in 0..64 {
            chip.execute(opcode)?;
        }
        assert_eq!(
            chip.execute(opcode),
            Err("Cannot enter subroutine, stack is full"),
        );

        chip.stack.clear();
        assert_eq!(
            chip.execute(OpCode::_2NNN { nnn: 0x100u16 }),
            Err("Attempted to jump out of program's address space"),
        );

        Ok(())
    }

    /// Skip the following instruction if the value of register VX equals NN
    #[test]
    fn execute_3xnn_skip_if_vx_eq_nn() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let pc = chip.pc;
        let opcode = OpCode::_3XNN { x: 0, nn: 0x22u8 };
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 2);

        chip.assign_vx_nn(0, 0x22u8)?;
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 6);
        Ok(())
    }

    /// Skip the following instruction if the value of register VX is not equal to NN
    #[test]
    fn execute_4xnn_skip_if_vx_ne_nn() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let pc = chip.pc;
        let opcode = OpCode::_4XNN { x: 0, nn: 0x22u8 };
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 4);

        chip.assign_vx_nn(0, 0x22u8)?;
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 6);
        Ok(())
    }

    /// Skip the following instruction if the value of register VX is equal to the value of register VY
    #[test]
    fn execute_5xy0_skip_if_vx_eq_vy() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let pc = chip.pc;
        let opcode = OpCode::_5XY0 { x: 0, y: 1 };
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 4);

        chip.assign_vx_nn(0, 0x22u8)?;
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 6);
        Ok(())
    }

    /// Store number NN in register VX
    #[test]
    fn execute_6xnn_assign_vx_nn() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::try_from(0x6122u16)?;
        chip.execute(opcode)?;
        assert_eq!(chip.v[1], 0x22u8);

        let opcode = OpCode::try_from(0x6FFFu16)?;
        chip.execute(opcode)?;
        assert_eq!(chip.v[15], 0xFFu8);
        Ok(())
    }

    /// Add the value NN to register VX
    #[test]
    fn execute_7xnn_assign_add_vx_nn() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let value = 0x09u8;
        let opcode = OpCode::_7XNN { x: 0, nn: value };
        // no flag should be set in VF during this execution
        chip.assign_vx_nn(0xFu8, value)?;

        chip.execute(opcode)?;
        assert_eq!(chip.v[0], value);
        assert_eq!(chip.v[15], value);

        chip.execute(opcode)?;
        assert_eq!(chip.v[0], value.wrapping_mul(2u8));
        assert_eq!(chip.v[15], value);
        Ok(())
    }

    /// Store the value of register VY in register VX
    #[test]
    fn execute_8xy0_assign_vx_vy() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let vx = 0x02u8;
        let vy = 0x04u8;
        let value = 0x09u8;

        chip.assign_vx_nn(vy, value)?;

        let opcode = OpCode::_8XY0 { x: vx, y: vy };
        chip.execute(opcode)?;
        assert_eq!(chip.v[vx as usize], value);
        Ok(())
    }

    /// Set VX to VX OR VY
    #[test]
    fn execute_8xy1_assign_or_vx_vy() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let vx = 0x02u8;
        let vy = 0x04u8;
        let value_x = 0xF1u8;
        let value_y = 0x0Fu8;

        chip.assign_vx_nn(vx, value_x)?;
        chip.assign_vx_nn(vy, value_y)?;

        let opcode = OpCode::_8XY1 { x: vx, y: vy };
        chip.execute(opcode)?;
        assert_eq!(chip.v[vx as usize], value_x | value_y);
        Ok(())
    }

    /// Set VX to VX AND VY
    #[test]
    fn execute_8xy2_assign_and_vx_vy() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let vx = 0x02u8;
        let vy = 0x04u8;
        let value_x = 0xF1u8;
        let value_y = 0x0Fu8;

        chip.assign_vx_nn(vx, value_x)?;
        chip.assign_vx_nn(vy, value_y)?;

        let opcode = OpCode::_8XY2 { x: vx, y: vy };
        chip.execute(opcode)?;
        assert_eq!(chip.v[vx as usize], value_x & value_y);
        Ok(())
    }

    /// Set VX to VX XOR VY
    #[test]
    fn execute_8xy3_assign_xor_vx_vy() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let vx = 0x02u8;
        let vy = 0x04u8;
        let value_x = 0xF1u8;
        let value_y = 0x1Fu8;

        chip.assign_vx_nn(vx, value_x)?;
        chip.assign_vx_nn(vy, value_y)?;

        let opcode = OpCode::_8XY3 { x: vx, y: vy };
        chip.execute(opcode)?;
        assert_eq!(chip.v[vx as usize], value_x ^ value_y);
        Ok(())
    }

    /// Add the value of register VY to register VX
    /// Set VF to 01 if a carry occurs
    /// Set VF to 00 if a carry does not occur
    #[test]
    fn execute_8xy4_assign_add_vx_vy() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let vx = 0x02u8;
        let vy = 0x04u8;
        let value = 0x8Fu8;

        chip.assign_vx_nn(vy, value)?;

        let opcode = OpCode::_8XY4 { x: vx, y: vy };
        chip.execute(opcode)?;
        assert_eq!(chip.v[vx as usize], value);
        assert_eq!(chip.v[15], 0x00u8);

        chip.execute(opcode)?;
        assert_eq!(chip.v[vx as usize], value.wrapping_mul(2));
        assert_eq!(chip.v[15], 0x01u8);
        Ok(())
    }

    /// Subtract the value of register VY from register VX
    /// Set VF to 00 if a borrow occurs
    /// Set VF to 01 if a borrow does not occur
    #[test]
    fn execute_8xy5_assign_sub_vx_vy() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let vx = 0x02u8;
        let vy = 0x04u8;
        let value_x = 0x05u8;
        let value_y = 0x04u8;

        chip.assign_vx_nn(vx, value_x)?;
        chip.assign_vx_nn(vy, value_y)?;

        let opcode = OpCode::_8XY5 { x: vx, y: vy };

        chip.execute(opcode)?;
        assert_eq!(chip.v[vx as usize], value_x.wrapping_sub(value_y));
        assert_eq!(chip.v[15], 0x01u8);

        chip.execute(opcode)?;
        assert_eq!(chip.v[vx as usize], value_x.wrapping_sub(2 * value_y));
        assert_eq!(chip.v[15], 0x00u8);
        Ok(())
    }

    /// Store the value of register VY shifted right one bit in register VX
    /// Set register VF to the least significant bit prior to the shift
    #[test]
    fn execute_8xy6_assign_vx_vy_shifted_r() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let vx = 0x02u8;
        let vy = 0x04u8;
        let value = 0b1111_1110u8;

        chip.assign_vx_nn(vy, value)?;

        let opcode = OpCode::_8XY6 { x: vx, y: vy };

        chip.execute(opcode)?;
        assert_eq!(chip.v[vy as usize], value >> 1);
        assert_eq!(chip.v[vx as usize], value >> 1);
        assert_eq!(chip.v[15], 0x00u8);

        chip.execute(opcode)?;
        assert_eq!(chip.v[vy as usize], value >> 2);
        assert_eq!(chip.v[vx as usize], value >> 2);
        assert_eq!(chip.v[15], 0x01u8);
        Ok(())
    }

    /// Set register VX to the value of VY minus VX
    /// Set VF to 00 if a borrow occurs
    /// Set VF to 01 if a borrow does not occur
    #[test]
    fn execute_8xy7_assign_vx_vy_sub_vx() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let vx = 0x02u8;
        let vy = 0x04u8;
        let value_x = 0x04u8;
        let value_y = 0x05u8;

        chip.assign_vx_nn(vx, value_x)?;
        chip.assign_vx_nn(vy, value_y)?;

        let opcode = OpCode::_8XY7 { x: vx, y: vy };

        chip.execute(opcode)?;
        assert_eq!(chip.v[vx as usize], value_y.wrapping_sub(value_x));
        assert_eq!(chip.v[15], 0x01u8);

        let value_x = value_y + 2;
        chip.assign_vx_nn(vx, value_x)?;
        chip.execute(opcode)?;
        assert_eq!(chip.v[vx as usize], value_y.wrapping_sub(value_x));
        assert_eq!(chip.v[15], 0x00u8);
        Ok(())
    }

    /// Store the value of register VY shifted left one bit in register VX
    /// Set register VF to the most significant bit prior to the shift
    #[test]
    fn execute_8xye_assign_vx_vy_shifted_l() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let vx = 0x02u8;
        let vy = 0x04u8;
        let value = 0b0111_1111u8;

        chip.assign_vx_nn(vy, value)?;

        let opcode = OpCode::_8XYE { x: vx, y: vy };

        chip.execute(opcode)?;
        assert_eq!(chip.v[vy as usize], value << 1);
        assert_eq!(chip.v[vx as usize], value << 1);
        assert_eq!(chip.v[15], 0x00u8);

        chip.execute(opcode)?;
        assert_eq!(chip.v[vy as usize], value << 2);
        assert_eq!(chip.v[vx as usize], value << 2);
        assert_eq!(chip.v[15], 0x01u8);
        Ok(())
    }

    /// Skip the following instruction if the value of register VX is not equal to the value of register VY
    #[test]
    fn execute_9xy0_skip_if_vx_ne_vy() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let pc = chip.pc;
        let opcode = OpCode::_9XY0 { x: 0, y: 1 };
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 2);

        chip.assign_vx_nn(0, 0x22u8)?;
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 6);
        Ok(())
    }

    /// Store memory address NNN in register I
    #[test]
    fn execute_annn_assign_i_nnn() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::_ANNN {
            nnn: MEM_LENGTH as u16 - 1,
        };
        assert_eq!(chip.i, 0x0000u16);
        chip.execute(opcode)?;
        assert_eq!(chip.i, MEM_LENGTH as u16 - 1);
        Ok(())
    }

    /// Jump to address NNN + V0
    #[test]
    fn execute_bnnn_jump_to_nnn_add_v0() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::try_from(0xB220u16)?;

        chip.execute(opcode)?;
        assert_eq!(chip.pc, 0x220u16);

        chip.assign_vx_nn(0, 0xFFu8)?;
        let opcode = OpCode::try_from(0xBF00u16)?;
        chip.execute(opcode)?;
        assert_eq!(chip.pc, MEM_LENGTH as u16 - 1);

        let opcode = OpCode::try_from(0xBFFBu16)?;
        assert_eq!(
            chip.execute(opcode),
            Err("Attempted to set pc out of address space"),
        );

        let opcode = OpCode::try_from(0xB000u16)?;
        assert_eq!(
            chip.execute(opcode),
            Err("Attempted to jump out of program's address space"),
        );
        Ok(())
    }

    /// Set VX to a random number with a mask of NN
    #[test]
    fn execute_cxnn_assign_vx_random_and_nn() -> Result<(), &'static str> {
        // Not testing this opcode currently, tested in test roms
        assert!(true);
        Ok(())
    }

    /// Draw a sprite at position VX VY with N bytes of sprite data starting at the address stored in I
    /// Set VF to 01 if any set pixels are changed to unset, and 00 otherwise
    #[rustfmt::skip]
    #[test]
    fn execute_dxyn_draw_n_at_vx_vy() -> Result<(), &'static str> {
        let mut chip = Peach8::load(TestingContext::new(0), &[]);
        let opcode = OpCode::_DXYN { x: 0, y: 1, n: 5 };

        chip.assign_vx_nn(0, 0x02)?;
        chip.assign_vx_nn(1, 0x01)?;
        chip.assign_vx_nn(2, 0x0F)?;
        chip.assign_i_addr_of_sprite_vx(2)?;
        chip.execute(opcode)?;
        assert_eq_2d!(
            x_range: 0..8, y_range: 0..8;
            chip.gfx.to_mask(), "........
                                 ..####..
                                 ..#.....
                                 ..####..
                                 ..#.....
                                 ..#.....
                                 ........
                                 ........".to_mask()
        );
        assert_eq!(chip.v[15], 0x00u8);

        chip.assign_vx_nn(0, 64u8 + 0x02)?;
        chip.assign_vx_nn(1, 32u8 + 0x01)?;
        chip.assign_vx_nn(2, 0x0E)?;
        chip.assign_i_addr_of_sprite_vx(2)?;
        chip.execute(opcode)?;
        assert_eq_2d!(
            x_range: 0..8, y_range: 0..8;
            chip.gfx.to_mask(), "........
                                 ........
                                 ........
                                 ........
                                 ........
                                 ...###..
                                 ........
                                 ........".to_mask()
        );
        assert_eq!(chip.v[15], 0x01u8);

        chip.assign_vx_nn(0, 62u8)?;
        chip.assign_vx_nn(1, 30u8)?;
        chip.assign_i_addr_of_sprite_vx(2)?;
        chip.execute(opcode)?;
        assert_eq_2d!(
            x_range: 60..64, y_range: 28..32;
            chip.gfx.to_mask(), "....
                                 ....
                                 ..##
                                 ..#.".to_mask().offset(60, 28)
        );
        assert_eq!(chip.v[15], 0x00u8);

        chip.assign_i_nnn(0x0FFEu16)?;
        assert_eq!(
            chip.execute(opcode),
            Err("Attempted to read memory out of address space"),
        );
        Ok(())
    }

    /// Skip the following instruction
    /// if the key corresponding to the hex value currently stored in register VX is pressed
    #[test]
    fn execute_ex9e_skip_if_vx_in_keys() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let pc = chip.pc;
        let opcode = OpCode::_EX9E { x: 0 };

        chip.assign_vx_nn(0, 0x0F)?;
        chip.update_keys();
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 2);

        chip.ctx.set_key(0x0Fu8);
        chip.update_keys();
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 6);

        chip.assign_vx_nn(0, 0xFF)?;
        chip.update_keys();
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 8);
        Ok(())
    }

    /// Skip the following instruction
    /// if the key corresponding to the hex value currently stored in register VX is not pressed
    #[test]
    fn execute_exa1_skip_if_vx_not_in_keys() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let pc = chip.pc;
        let opcode = OpCode::_EXA1 { x: 0 };

        chip.assign_vx_nn(0, 0x0F)?;
        chip.update_keys();
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 4);

        chip.ctx.set_key(0x0Fu8);
        chip.update_keys();
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 6);

        chip.assign_vx_nn(0, 0xFF)?;
        chip.update_keys();
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 10);
        Ok(())
    }

    /// Store the current value of the delay timer in register VX
    #[test]
    fn execute_fx07_assign_vx_delay_t() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::_FX07 { x: 0 };
        chip.delay_timer.store(0xFFu8);

        chip.execute(opcode)?;
        assert_eq!(chip.delay_timer.load(), chip.v[0]);
        Ok(())
    }

    /// Wait for a keypress and store the result in register VX
    /// Should trigger on key release
    #[test]
    fn execute_fx0a_assign_vx_wait_for_key() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let pc = chip.pc;
        let opcode = OpCode::_FX0A { x: 0 };

        chip.update_keys();
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc);
        assert_eq!(chip.v[0], 0x00u8);

        chip.ctx.set_key(0x0Fu8);
        chip.update_keys();
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc);
        assert_eq!(chip.v[0], 0x00u8);

        chip.ctx.reset_key(0x0Fu8);
        chip.update_keys();
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 2);
        assert_eq!(chip.v[0], 0x0Fu8);

        chip.assign_vx_nn(0, 0x00)?;
        chip.update_keys();
        chip.execute(opcode)?;
        assert_eq!(chip.pc, pc + 2);
        assert_eq!(chip.v[0], 0x00u8);
        Ok(())
    }

    /// Set the delay timer to the value of register VX
    #[test]
    fn execute_fx15_assign_delay_t_vx() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::_FX15 { x: 0 };
        chip.assign_vx_nn(0, 0xFFu8)?;

        chip.execute(opcode)?;
        assert_eq!(chip.delay_timer.load(), chip.v[0]);
        Ok(())
    }

    /// Set the sound timer to the value of register VX
    #[test]
    fn execute_fx18_assign_sound_t_vx() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::_FX18 { x: 0 };
        chip.assign_vx_nn(0, 0xFFu8)?;

        chip.execute(opcode)?;
        assert_eq!(chip.sound_timer.load(), chip.v[0]);
        Ok(())
    }

    /// Add the value stored in register VX to register I
    #[test]
    fn execute_fx1e_assign_add_i_vx() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::_FX1E { x: 0 };

        chip.execute(opcode)?;
        assert_eq!(chip.i, 0x0000u16);

        chip.assign_vx_nn(0, 0xFFu8)?;
        chip.execute(opcode)?;
        assert_eq!(chip.i, 0x00FFu16);

        chip.assign_i_nnn(0x0FFBu16)?;
        assert_eq!(
            chip.execute(opcode),
            Err("Attempted to set i out of address space"),
        );
        Ok(())
    }

    /// Set I to the memory address of the sprite data
    /// corresponding to the hexadecimal digit stored in register VX
    #[test]
    fn execute_fx29_assign_i_addr_of_sprite_vx() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::_FX29 { x: 0 };

        chip.assign_vx_nn(0, 0x00u8)?;
        chip.execute(opcode)?;
        assert_eq!(chip.i, FONTSET_ADDR);

        chip.assign_vx_nn(0, 0xACu8)?;
        chip.execute(opcode)?;
        assert_eq!(chip.i, FONTSET_ADDR + 0xC * 5);

        chip.assign_vx_nn(0, 0xB7u8)?;
        chip.execute(opcode)?;
        assert_eq!(chip.i, FONTSET_ADDR + 0x7 * 5);
        Ok(())
    }

    /// Store the binary-coded decimal equivalent of the value
    /// stored in register VX at addresses I, I+1, and I+2
    #[test]
    fn execute_fx33_assign_mem_at_i_bcd_of_vx() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));
        let opcode = OpCode::_FX33 { x: 0 };

        chip.execute(opcode)?;
        assert_eq!(
            &chip.memory[chip.i as usize..=(chip.i + 2) as usize],
            &[0, 0, 0],
        );

        chip.assign_vx_nn(0, 0xFFu8)?;
        chip.execute(opcode)?;
        assert_eq!(
            &chip.memory[chip.i as usize..=(chip.i + 2) as usize],
            &[2, 5, 5],
        );

        chip.assign_i_nnn((MEM_LENGTH - 1) as u16)?;
        assert_eq!(
            chip.execute(opcode),
            Err("Attempted to set memory out of address space"),
        );
        Ok(())
    }

    /// Store the values of registers V0 to VX inclusive in memory
    /// starting at address I, I is set to I + X + 1 after operation
    #[test]
    fn execute_fx55_assign_mem_at_i_v0_to_vx() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));

        chip.assign_vx_nn(0, 0xDEu8)?;
        chip.assign_vx_nn(1, 0xADu8)?;
        chip.assign_vx_nn(2, 0xBEu8)?;
        chip.assign_vx_nn(3, 0xEFu8)?;

        let opcode = OpCode::_FX55 { x: 0 };
        chip.execute(opcode)?;
        assert_eq!(chip.memory[(chip.i - 1) as usize], 0xDEu8);
        assert_eq!(chip.i, 0x0001u16);

        let opcode = OpCode::_FX55 { x: 3 };
        chip.execute(opcode)?;
        assert_eq!(
            &chip.memory[(chip.i - 4) as usize..chip.i as usize],
            &[0xDE, 0xAD, 0xBE, 0xEF],
        );
        assert_eq!(chip.i, 0x0005u16);

        let opcode = OpCode::_FX55 { x: 0x0Fu8 };
        chip.assign_i_nnn(0x0FF1u16)?;
        assert_eq!(
            chip.execute(opcode),
            Err("Attempted to store data out of address space"),
        );
        Ok(())
    }

    /// Fill registers V0 to VX inclusive with the values stored in memory
    /// starting at address I, I is set to I + X + 1 after operation
    #[test]
    fn execute_fx65_assign_v0_to_vx_mem_at_i() -> Result<(), &'static str> {
        let mut chip = Peach8::new(TestingContext::new(0));

        chip.memory[chip.i as usize] = 0xDEu8;
        chip.memory[(chip.i + 1) as usize] = 0xADu8;
        chip.memory[(chip.i + 2) as usize] = 0xBEu8;
        chip.memory[(chip.i + 3) as usize] = 0xEFu8;

        let opcode = OpCode::_FX65 { x: 3 };
        chip.execute(opcode)?;
        assert_eq!(chip.v[0], 0xDEu8);
        assert_eq!(chip.v[1], 0xADu8);
        assert_eq!(chip.v[2], 0xBEu8);
        assert_eq!(chip.v[3], 0xEFu8);
        assert_eq!(chip.i, 0x0004u16);

        let opcode = OpCode::_FX65 { x: 0x0Fu8 };
        chip.assign_i_nnn(0x0FF1u16)?;
        assert_eq!(
            chip.execute(opcode),
            Err("Attempted to load memory out of address space"),
        );
        Ok(())
    }
}
