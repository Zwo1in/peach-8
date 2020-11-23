//! # Chip 8
//! This crate aims to provide platform agnostic implementation
//! of [Chip-8 interpreter](https://en.wikipedia.org/wiki/CHIP-8).
//!
//! There are alot of extensions existing for Chip-8 platform, however
//! this crate will provide only the most common 35 instruction from
//! Super-Chip specification from 1991, without the additional opcodes
//! that provide extended functionality.
//!
//! Fully qualified emulator should consist of following peripherals:
//! - sound sink (most commonly a buzzer)
//! - 64x32 px display
//! - 4x4 matrix keyboard
//!
//! # No std
//! Peach8 is fully compatible with `no_std` environment.
//! To overcome the differences between different embedded platforms,
//! the user is provided with a `Context` trait, which should handle:
//! - displaying frame on the screen,
//! - getting user input,
//! - generating random numbers,
//! - turning sound on and off,
//!
//! # Implementation guidelines
//! There are two main methods that progresses emulation:
//! - `Peach8::tick_chip`
//! - `Peach8::tick_timers`
//!
//! Both of them should be tacted independly. `tick_timers` should be called
//! with 60Hz frequency, and `tick_chip` should be called with around 500Hz
//! frequency.
//!
//! Emulation cycle (`tick_chip`) is as follows:
//! - Get input (`Context::get_keys`),
//! - Execute next instruction,
//! - Draw to the screen (`Context::on_frame`),
//!
//! Timers cycle (`tick_timers`) is as follows:
//! - Decrement active timers (sound and delay),
//! - Call `Context::sound_on` or `Context::sound_off` when appropriate,
//!
//! # Thread safety
//! Although most `no_std` targets are single-threaded, the interrupts may
//! lead to the same problems that are encountered in multi-threading.
//!
//! `Peach8` can handle thread-safety on targets, where there is a support for
//! atomic operations on `u8`. This is handled by `atomic` feature, enabled by
//! default. On platforms where there is no such support, eg. `thumbv6em-none-eabi`,
//! implementation should ensure that those methods won`t interrupt eachother.
//!
//! Implementation of `Context` trait also have to be `Sync` for `Peach8` to be sync.
//!
//! # Examples:
//! coming soon...

#![no_std]
pub mod builder;
pub mod context;
pub mod frame;
pub mod opcode;
pub mod peach;
pub(crate) mod timer;
pub(crate) mod utils;

pub use builder::Builder;
pub use context::Context;
#[cfg(feature = "embedded-graphics")]
pub use embedded_graphics;
pub use frame::{Frame, FrameView};
pub use peach::Peach8;
