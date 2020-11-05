#![allow(dead_code)]

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimerState {
    On,
    Off,
    Finished,
}

pub mod racy {
    use super::TimerState;

    #[derive(Debug)]
    pub struct Timer(u8);

    impl Timer {
        pub fn new() -> Self {
            Self(0)
        }

        #[inline]
        pub fn store(&mut self, value: u8) {
            self.0 = value;
        }

        #[inline]
        pub fn load(&self) -> u8 {
            self.0
        }

        #[inline]
        pub fn decrement(&mut self) -> TimerState {
            if self.0 > 0 {
                self.0 -= 1;
                if self.0 == 0 {
                    TimerState::Finished
                } else {
                    TimerState::On
                }
            } else {
                TimerState::Off
            }
        }
    }
}

#[cfg(feature = "atomic")]
pub mod atomic {
    use super::TimerState;
    use core::sync::atomic::{AtomicU8, Ordering};

    #[derive(Debug)]
    pub struct Timer(AtomicU8);

    impl Timer {
        pub fn new() -> Self {
            Self(AtomicU8::new(0))
        }

        #[inline]
        pub fn store(&mut self, value: u8) {
            self.0.store(value, Ordering::Release);
        }

        #[inline]
        pub fn load(&self) -> u8 {
            self.0.load(Ordering::Acquire)
        }

        #[inline]
        pub fn decrement(&mut self) -> TimerState {
            self.0
                .fetch_update(Ordering::Release, Ordering::Relaxed, |value| {
                    if value > 0 {
                        Some(value - 1)
                    } else {
                        Some(value)
                    }
                })
                .map(|value| match value {
                    0 => TimerState::Off,
                    1 => TimerState::Finished,
                    _ => TimerState::On,
                })
                .unwrap()
        }
    }
}
