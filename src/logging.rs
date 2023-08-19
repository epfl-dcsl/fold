use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

use log::{LevelFilter, Metadata, Record};
use rustix::{io, stdio};

// ———————————————————————————————— Println ————————————————————————————————— //

pub struct Stdout {}

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let stdout = unsafe { stdio::stdout() };
        io::write(stdout, s.as_bytes()).unwrap();
        Ok(())
    }
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        {
            use ::core::fmt::Write;
            ::core::writeln!($crate::logging::Stdout {}, $($arg)*).unwrap();
        }
    }
}

// ————————————————————————————————— Logger ————————————————————————————————— //

struct Logger {}

static LOGGER: Logger = Logger {};
static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);

impl log::Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        println!("[{}] {}", record.level(), record.args());
    }

    fn flush(&self) {}
}

pub fn init(level: LevelFilter) {
    match IS_INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => {
            log::set_logger(&LOGGER).unwrap();
            log::set_max_level(level);
        }
        Err(_) => {
            log::warn!("Logger is already initialized, skipping init");
        }
    };
}
