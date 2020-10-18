//! Provides helper functions to create and initialize logging interfaces
//! available through cortex_m_log crate.
//!
//! Available logging interfaces:
//! - Semihosting STDOUT
//! - Semihosting STDERR
//! - Instrumentation Trace Macrocell
//!
//! Available critical section manip
//! - InterruptFree: logging calls are executed in interrupt free context
//! - InterruptOk: logging calls may be interrupted
//!
//! Provided loggers are available to `log` facade after calling unsafe init function
//!
//! # Examples
//!
//! ```no_run
//! # use peripherals::logger::*;
//! # use log::{info, debug};
//! #
//! # let r0 = 15;
//! #
//! let p = cortex_m::Peripherals::take().unwrap();
//!
//! let logger = create_itm_logger::<InterruptFree>(LevelFilter::max(), p.ITM);
//! unsafe {
//!     init(&logger);
//! }
//!
//! info!("Shutdown procedure started");
//! debug!("Value of r0: {}", r0);
//! ```

use core::marker::{Send, Sync};
use cortex_m::peripheral::ITM;
use cortex_m_log::{
    destination,
    log::{trick_init, Logger},
    modes::InterruptModer,
    printer::{
        itm::ItmSync,
        semihosting::{
            hio::{HStderr, HStdout},
            Semihosting,
        },
        Printer,
    },
};

pub use log::LevelFilter;

pub use cortex_m_log::modes::{InterruptFree, InterruptOk};

/// Create new logger instance with ITM backend
///
/// Requires enabling ITM in openocd
///
/// # Examples
///
/// gdb:
/// ```gdb
/// monitor tpiu config internal itm.out uart off 8000000
/// monitor itm port 0 on
/// ```
///
/// shell:
/// ```sh
/// itmdump -F -f itm.out
/// ```
///
/// ```no_run
/// use peripherals::logger::{
///     create_itm_logger,
///     InterruptFree,
///     LevelFilter,
///     init,
/// };
///
/// use log::info;
///
/// let p = cortex_m::Peripherals::take().unwrap();
/// let logger = create_itm_logger::<InterruptFree>(LevelFilter::max(), p.ITM);
/// unsafe { init(&logger); }
///
/// info!("Hello world");
/// ```
pub fn create_itm_logger<M>(level: LevelFilter, itm_reg: ITM) -> Logger<ItmSync<M>>
where
    M: InterruptModer + Send + Sync + 'static,
{
    Logger {
        level,
        inner: ItmSync::<M>::new(destination::Itm::new(itm_reg)),
    }
}

/// Create new logger instance with semihosting backend writing to host's stdout
///
/// Requires enabling semihosting in openocd
///
/// # Examples
///
/// gdb:
/// ```gdb
/// monitor arm semihosting enable
/// ```
///
/// ```no_run
/// use peripherals::logger::{
///     create_shout_logger,
///     InterruptFree,
///     LevelFilter,
///     init,
/// };
///
/// use log::info;
///
/// let logger = create_shout_logger::<InterruptFree>(LevelFilter::max());
/// unsafe { init(&logger); }
///
/// info!("Hello world");
/// ```
pub fn create_shout_logger<M>(level: LevelFilter) -> Logger<Semihosting<M, HStdout>>
where
    M: InterruptModer + Send + Sync + 'static,
{
    Logger {
        level,
        inner: Semihosting::<M, _>::stdout().expect("Failed to retreive semihosting stdout"),
    }
}

/// Create new logger instance with semihosting backend writing to host's stderr
///
/// Requires enabling semihosting in openocd ['create_shout_logger']
///
/// # Examples
///
/// gdb:
/// ```gdb
/// monitor arm semihosting enable
/// ```
///
/// ```no_run
/// use peripherals::logger::{
///     create_sherr_logger,
///     InterruptFree,
///     LevelFilter,
///     init,
/// };
///
/// use log::info;
///
/// let logger = create_sherr_logger::<InterruptFree>(LevelFilter::max());
/// unsafe { init(&logger); }
///
/// info!("Hello world");
/// ```
pub fn create_sherr_logger<M>(level: LevelFilter) -> Logger<Semihosting<M, HStderr>>
where
    M: InterruptModer + Send + Sync + 'static,
{
    Logger {
        level,
        inner: Semihosting::<M, _>::stderr().expect("Failed to retreive semihosting stderr"),
    }
}

/// Initialize logger for the log facade.
///
/// # Safety
///
/// This function should only be called once
///
/// This function mutates lifetime of a logger, tricking compiler that it's lifetime is static
///
/// # Undefined Behaviour
///
/// It is UB to drop logger instance and try to use logging API afterwards.
/// It's user's responsibility to keep an instance valid
///
/// This definitely won't print anything, and no prints are the best scenario in this case:
/// ```no_run
/// # use peripherals::logger::{
/// #     create_sherr_logger,
/// #     InterruptFree,
/// #     LevelFilter,
/// #     init,
/// # };
/// # use log::info;
/// {
///     let logger = create_sherr_logger::<InterruptFree>(LevelFilter::max());
///     unsafe { init(&logger); }
/// }
///
/// info!("Hello world");
/// ```
pub unsafe fn init<P>(logger: &Logger<P>)
where
    P: Printer + Send + Sync + 'static,
{
    trick_init(logger).expect("Failed to initialize logger");
}
