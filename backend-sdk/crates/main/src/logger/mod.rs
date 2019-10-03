/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! This module enables log messages from the Wasm side. It is implemented as a logging facade for
//! crate [`log`]. To enable this module in your project please specify `wasm_logger` feature of
//! `fluence_sdk`.
//!
//! Note that this module works only for the Wasm environments and Fluence `WasmVm` - with this
//! feature set it is possible to compile applications only for Wasm targets such as
//! `wasm32-unknown-unknown`, `wasm32-wasi`. (please refer to the first example to find out a way
//! to avoid it).
//!
//! This feature should be used only for debugging purposes, you can find more info in the
//! [`backend app debugging`] section of the Fluence guide.
//!
//! # Examples
//!
//! This example initializes [`WasmLogger`] if target arch is Wasm and [`simple_logger`] otherwise.
//! Macros from crate [`log`] are used as a logging facade.
//!
//! ```
//!     use fluence::sdk::*;
//!     use log::{error, trace};
//!     use simple_logger;
//!
//!     fn main() {
//!         if cfg!(target_arch = "wasm32") {
//!             logger::WasmLogger::init_with_level(log::Level::Info).unwrap();
//!         } else {
//!             simple_logger::init_with_level(log::Level::Info).unwrap();
//!         }
//!
//!         error!("This message will be logged.");
//!         trace!("This message will not be logged.");
//!     }
//!
//! ```
//!
//! This example provides methods for [`WasmLogger`] initialization only for Wasm target without
//! specifying logger level:
//!
//! ```
//!     use fluence::sdk::*;
//!     use log::info;
//!
//!     /// This method initializes WasmLogger and should be called at the start of the application.
//!     #[no_mangle]
//!     #[cfg(target_arch = "wasm32")]
//!     fn init_logger() {
//!         logger::WasmLogger::init().unwrap();
//!         info!("If you can see this message that logger was successfully initialized.");
//!     }
//!
//! ```
//!
//! [`WasmLogger`]: struct.WasmLogger.html
//! [`log`]: https://docs.rs/log
//! [`simple_logger`]: https://docs.rs/simple_logger
//! [`static_lazy`]: https://docs.rs/lazy_static
//! [`lazy_static::initialize()`]: https://docs.rs/lazy_static/1.3.0/lazy_static/fn.initialize.html
//! [`backend app debugging`]: https://fluence.dev/docs/debugging

extern crate log;

/// The Wasm Logger.
///
/// This struct implements the [`Log`] trait from the [`log`] crate, which allows it to act as a
/// logger.
///
/// For initialization of WasmLogger as a default logger please see [`init()`]
/// and [`init_with_level()`]
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
    /// # use fluence::sdk::*;
    /// # use log::info;
    /// #
    /// # fn main() {
    /// if cfg!(target_arch = "wasm32") {
    ///     logger::WasmLogger::init_with_level(log::Level::Error).unwrap();
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

    /// Initializes the global logger with a [`WasmLogger`] instance with `max_log_level` set to
    /// `Level::Info`.
    ///
    /// ```
    /// # use fluence::sdk::*;
    /// # use log::info;
    /// #
    /// # fn main() {
    /// if cfg!(target_arch = "wasm32") {
    ///     fluence::logger::WasmLogger::init().unwrap();
    /// }
    ///
    /// error!("This message will be logged.");
    /// trace!("This message will not be logged.");
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

        log(log_msg.as_ptr() as i32, log_msg.len() as i32);
    }
}

/// logger is a module provided by a VM that can process log messages.
#[link(wasm_import_module = "logger")]
extern "C" {
    // Writes a byte string of size bytes that starts from ptr to a logger
    fn log(ptr: i32, size: i32);
}
