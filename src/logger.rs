use cortex_m::peripheral::ITM;
use cortex_m_log::{
    println,
    printer::Itm,
    destination,
    modes::InterruptFree,
};

pub enum Severity {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Severity::Error => "ERROR",
            Severity::Warn  => "WARN",
            Severity::Debug => "DEBUG",
            Severity::Info  => "INFO",
            Severity::Trace => "TRACE",
        }
    }
}


pub struct Logger {
    itm: Itm<InterruptFree>,
}

impl Logger {
    pub fn new(itm: ITM) -> Self {
        let itm = Itm::new(destination::Itm::new(itm));
        Self { itm }
    }

    pub fn log(&mut self, sev: Severity, modpath: &str, what: core::fmt::Arguments) {
        println!(self.itm, "[{:5}][{}] {}", sev.as_str(), modpath, what);
    }

    pub fn error(&mut self, modpath: &str, what: core::fmt::Arguments) {
        self.log(Severity::Error, modpath, what);
    }

    pub fn warn(&mut self, modpath: &str, what: core::fmt::Arguments) {
        self.log(Severity::Warn, modpath, what);
    }

    pub fn info(&mut self, modpath: &str, what: core::fmt::Arguments) {
        self.log(Severity::Info, modpath, what);
    }

    pub fn debug(&mut self, modpath: &str, what: core::fmt::Arguments) {
        self.log(Severity::Debug, modpath, what);
    }

    pub fn trace(&mut self, modpath: &str, what: core::fmt::Arguments) {
        self.log(Severity::Trace, modpath, what);
    }
}

#[macro_export]
macro_rules! error {
    ($logger:ident, $($args:tt),+) => {
        $logger.borrow_mut().error(module_path!(), format_args!($($args),+)); 
    }
}

#[macro_export]
macro_rules! warn {
    ($logger:ident, $($args:tt),+) => {
        $logger.borrow_mut().warn(module_path!(), format_args!($($args),+)); 
    }
}

#[macro_export]
macro_rules! info {
    ($logger:ident, $($args:tt),+) => {
        $logger.borrow_mut().info(module_path!(), format_args!($($args),+)); 
    }
}

#[macro_export]
macro_rules! debug {
    ($logger:ident, $($args:tt),+) => {
        $logger.borrow_mut().debug(module_path!(), format_args!($($args),+)); 
    }
}

#[macro_export]
macro_rules! trace {
    ($logger:ident, $($args:tt),+) => {
        $logger.borrow_mut().trace(module_path!(), format_args!($($args),+)); 
    }
}
