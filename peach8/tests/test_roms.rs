use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use crossbeam_utils::thread;

use peach8::{
    embedded_graphics::{
        image::{ImageRaw, IntoPixelIter},
        pixelcolor::BinaryColor,
    },
    Context, Peach8,
};

macro_rules! schedule_for {
    ($scope:expr, $f:expr, $freq:expr, $timeout:expr) => {{
        let started = Instant::now();
        let period = Duration::from_nanos(1_000_000_000u64 / $freq);
        let mut previous = started;
        $scope.spawn(move |_| loop {
            let now = Instant::now();
            if now.duration_since(started) >= $timeout {
                break;
            }
            if now.duration_since(previous) >= period {
                $f();
                previous = now;
            }
        })
    }};
}

#[ignore]
#[test]
fn scheduler_tests() {
    let counter = Arc::new(AtomicUsize::new(0));
    thread::scope(|s| {
        let counter_cln = Arc::clone(&counter);
        schedule_for!(
            s,
            || {
                counter_cln.fetch_add(1, Ordering::Relaxed);
            },
            10,
            Duration::from_secs(3)
        );
    })
    .unwrap();
    assert_eq!(counter.load(Ordering::Relaxed), 29);
}

struct TestingContext(Vec<String>);

impl TestingContext {
    fn new() -> Self {
        let mut row = String::new();
        for _ in 0..64 {
            row.push('.');
        }
        let mut inner = vec![];
        inner.resize_with(32, || row.clone());
        Self(inner)
    }

    fn formatted(&self) -> String {
        self.0.join("\n") + "\n"
    }
}

impl Context for TestingContext {
    fn on_frame<'a>(&mut self, frame: ImageRaw<'a, BinaryColor>) {
        frame.pixel_iter().for_each(|px| {
            let (x, y) = (px.0.x as usize, px.0.y as usize);
            self.0[y].replace_range(
                x..x + 1,
                match px.1 {
                    BinaryColor::On => "#",
                    BinaryColor::Off => ".",
                },
            );
        });
    }

    fn sound_on(&mut self) {}

    fn sound_off(&mut self) {}

    fn get_keys(&mut self) -> &[bool; 16] {
        &[false; 16]
    }

    fn gen_random(&mut self) -> u8 {
        rand::random::<u8>()
    }
}

/// Not working currently as using modern opcode's behaviours. For future impl of compatibility
/// flags
///
/// TEST ORDER
/// 0: 3XNN
/// 1: 4XNN
/// 2: 5XY0
/// 3: 7XNN (not carry flag and overflow value)
/// 4: 8XY0
/// 5: 8XY1
/// 6: 8XY2
/// 7: 8XY3
/// 8: 8XY4
/// 9: 8XY5
/// 10: 8XY6
/// 12: 8XY7
/// 12: 8XYE
/// 13: 9XY0
/// 14: BNNN
/// 15: CXNN  Note: Always a small chance of failure if(rand() == rand()) { fail }
/// 16: FX07  Note: If fail it may be because either FX15 or FX07 fails or because delay_timer is
///                 not implemented. If the the emulation is too fast this might also fail.
/// 17:FX33/FX65/ANNN
/// 18:FX55/FX65
/// 19: FX1E
#[ignore]
#[test]
fn rom_skosulor_c8int() {
    let _ = env_logger::builder().is_test(true).try_init();

    let rom = include_bytes!("../test-data/skosulor_c8int/test.c8");
    let chip = Arc::new(Mutex::new(Peach8::load(TestingContext::new(), &rom[..])));
    let chip_timers = Arc::clone(&chip);
    let chip_test = Arc::clone(&chip);
    thread::scope(|s| {
        schedule_for!(
            s,
            || chip.lock().unwrap().tick_chip().unwrap(),
            500,
            Duration::from_millis(300)
        );
        schedule_for!(
            s,
            || chip_timers.lock().unwrap().tick_timers(),
            60,
            Duration::from_millis(300)
        );
    })
    .unwrap();

    let lhs = chip_test.lock().unwrap().ctx.formatted();
    let rhs = include_str!("../test-data/context/empty_mask");
    assert_eq!(&lhs, rhs, "\nlhs:\n{}\n\nrhs:\n{}", lhs, rhs,);
}

#[test]
fn rom_corax89_chip8_test_rom() {
    let _ = env_logger::builder().is_test(true).try_init();

    let rom = include_bytes!("../test-data/corax89_chip8-test-rom/test_opcode.ch8");
    let chip = Arc::new(Mutex::new(Peach8::load(TestingContext::new(), &rom[..])));
    let chip_timers = Arc::clone(&chip);
    let chip_test = Arc::clone(&chip);
    thread::scope(|s| {
        schedule_for!(
            s,
            || chip.lock().unwrap().tick_chip().unwrap(),
            500,
            Duration::from_millis(500)
        );
        schedule_for!(
            s,
            || chip_timers.lock().unwrap().tick_timers(),
            60,
            Duration::from_millis(500)
        );
    })
    .unwrap();

    let lhs = chip_test.lock().unwrap().ctx.formatted();
    let rhs = include_str!("../test-data/corax89_chip8-test-rom/expected_result");
    assert_eq!(&lhs, rhs, "\nlhs:\n{}\n\nrhs:\n{}", lhs, rhs,);
}
