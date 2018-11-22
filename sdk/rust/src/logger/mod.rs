//! This module enables log messages writes from the WASM code.
//!
//! This is a client for the Logger Wasm Module in Fluence WasmVm. Together they
//! allow to write messages from Wasm code to log. The logging is basically a
//! 'stdout' and depends on Vm implementation. By default this module is disabled
//! in WasmVm.
//!
//! Note that this module works only for the Wasm environment and Fluence WasmVm.
//! Don't use it for other targets and Vms.
//!
//! This module provides implementation for logging facade in crate [`log`].
//! See examples below:
//!
//! # Examples
//!
//! This example initializes [`WasmLogger`] only for Wasm target, for another
//! targets initializes [`simple_logger`]. Macroses from crate [`log`] used as
//! logging facade.
//!
//! ```
//!     #[macro_use] extern crate log;
//!     extern crate fluence_sdk;
//!     extern crate simple_logger;
//!
//!     fn main() {
//!         if cfg!(target_arch = "wasm32") {
//!             fluence_sdk::logger::WasmLogger::init_with_level(log::Level::Info).unwrap();
//!         } else {
//!             simple_logger::init_with_level(log::Level::Info).unwrap();
//!         }
//!
//!         error!("This message will be logged.");
//!         trace!("This message will not be logged.");
//!     }
//!
//! ```
//! This example provides method for initialization [`WasmLogger`] only for Wasm
//! target without specifying logger level. Macroses from crate [`log`] used as
//! logging facade.
//!
//! ```
//!     #[macro_use] extern crate log;
//!     extern crate fluence_sdk;
//!
//!     /// This method initialize WasmLogger and should be called at the start of application
//!     #[no_mangle]
//!     #[cfg(target_arch = "wasm32")]
//!     fn init_logger() {
//!         fluence_sdk::logger::WasmLogger::init().unwrap();
//!         info!("If you can see this message that logger was successfully initialized.");
//!     }
//!
//! ```
//! You can also use [`static_lazy`] for [`WasmLogger`] initialization but laziness
//! has some caveats. You need to call [`lazy_static::initialize()`] for
//! eager initialization before first logger macros usage.
//!
//! ```
//!     #[macro_use] extern crate log;
//!     #[macro_use] extern crate lazy_static;
//!     extern crate fluence_sdk;
//!
//!     lazy_static! {
//!         static ref _LOGGER: () = {
//!             fluence_sdk::logger::WasmLogger::init_with_level(log::Level::Info);
//!         };
//!     }
//!
//!     fn main() {
//!         if cfg!(target_arch = "wasm32") {
//!             // There is required to call init in a method or in another `lazy_static!` block
//!             fluence_sdk::logger::WasmLogger::init_with_level(log::Level::Info).unwrap();
//!         }
//!
//!         // ...
//!     }
//!
//! ```
//! [`WasmLogger`]: struct.WasmLogger.html
//! [`log`]: https://docs.rs/log
//! [`simple_logger`]: https://docs.rs/simple_logger
//! [`static_lazy`]: https://docs.rs/lazy_static
//! [`lazy_static::initialize()`]: https://docs.rs/lazy_static/1.2.0/lazy_static/fn.initialize.html

extern crate log;

/// The Wasm Logger.
///
/// This struct implements the [`Log`] trait from the [`log`] crate,
/// which allows it to act as a logger.
///
/// For initialization WasmLogger as default logger see [`init()`] and [`init_with_level()`]
///
/// [log-crate-url]: https://docs.rs/log/
/// [`Log`]: https://docs.rs/log/0.4.6/log/trait.Log.html
/// [`init_with_level()`]: struct.WasmLogger.html#method.init_with_level
/// [`init()`]: struct.WasmLogger.html#method.init
pub struct WasmLogger {
    level: log::Level,
}

impl WasmLogger {
    /// Initializes the global logger with a [`WasmLogger`] instance with
    /// `max_log_level` set to a specific log level.
    ///
    /// ```
    /// # #[macro_use] extern crate log;
    /// # extern crate fluence_sdk;
    /// #
    /// # fn main() {
    /// if cfg!(target_arch = "wasm32") {
    ///     fluence_sdk::logger::WasmLogger::init_with_level(log::Level::Error).unwrap();
    /// }
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

    /// Initializes the global logger with a [`WasmLogger`] instance with
    /// `max_log_level` set to `Level::Trace`.
    ///
    /// ```
    /// # #[macro_use] extern crate log;
    /// # extern crate fluence_sdk;
    /// #
    /// # fn main() {
    /// if cfg!(target_arch = "wasm32") {
    ///     fluence_sdk::logger::WasmLogger::init().unwrap();
    /// }
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

    #[inline]
    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let log_msg = format!(
            "{:<5} [{}] {}\n",
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

/// Wasm module for writing logs from Wasm code, provided by WasmVm.
#[link(wasm_import_module = "logger")]
extern "C" {

    /// Writes one byte to logger inner state.
    fn write(byte: i32);

    /// Flush all logger inner state to log.
    fn flush();
}
