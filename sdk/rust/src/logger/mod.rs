//! Module for integrating with Logger Wasm Module in Fluence WasmVm. By default
//! this module is disabled.
//!
//! # Examples
//!
//! todo write docs and examples

extern crate log;

/// The Wasm Logger.
///
/// This struct implements the `Log` trait from the [`log` crate][log-crate-url],
/// which allows it to act as a logger.
///
/// For initialization WasmLogger as default logger see [`init()`] and [`init_with_level()`]
///
/// [log-crate-url]: https://docs.rs/log/
/// [`init_with_level()`]: struct.WasmLogger.html#method.init_with_level
/// [`init()`]: struct.WasmLogger.html#method.init
pub struct WasmLogger {
    level: log::Level,
}

impl WasmLogger {
    /// Initializes the global logger with a WasmLogger instance with
    /// `max_log_level` set to a specific log level.
    ///
    /// ```
    /// # #[macro_use] extern crate log;
    /// # extern crate fluence_sdk;
    /// #
    /// # fn main() {
    /// fluence_sdk::logger::WasmLogger::init_with_level(log::Level::Error).unwrap();
    ///
    /// error!("This message will be logged.");
    /// info!("This message will not be logged.");
    /// # }
    /// ```
    pub fn init_with_level(level: log::Level) -> Result<(), log::SetLoggerError> {
        let logger = WasmLogger { level };
        log::set_boxed_logger(Box::new(logger))?;
        log::set_max_level(level.to_level_filter());
        Ok(())
    }

    /// Initializes the global logger with a WasmLogger instance with
    /// `max_log_level` set to `Level::Trace`.
    ///
    /// ```
    /// # #[macro_use] extern crate log;
    /// # extern crate fluence_sdk;
    /// #
    /// # fn main() {
    /// fluence_sdk::logger::WasmLogger::init().unwrap();
    ///
    /// error!("This message will be logged.");
    /// trace!("This message will not be logged too.");
    /// # }
    /// ```
    pub fn init() -> Result<(), log::SetLoggerError> {
        WasmLogger::init_with_level(log::Level::Info)
    }
}

impl log::Log for WasmLogger {
    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let log_msg = format!(
            "{:<5} [{}] {}",
            record.level().to_string(),
            record.module_path().unwrap_or_default(),
            record.args()
        );

        unsafe {
            for byte in log_msg.as_bytes() {
                write(i32::from(*byte))
            }
        }

        self.flush()
    }

    #[inline]
    fn flush(&self) {
        unsafe { flush() };
    }
}

// todo use cfg_if!

/// Wasm module provided by WasmVm for writing log from Wasm code.
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "logger")]
extern "C" {

    /// Writes one byte to logger inner state.
    fn write(byte: i32);

    /// Flush all logger inner state to log.
    fn flush();
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn write(_byte: i32) {
    // noop
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn flush() {
    // noop
}
