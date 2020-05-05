/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use std::error::Error;
use wasmer_runtime::error::{
    CallError, CompileError, CreationError, Error as WasmerError, ResolveError, RuntimeError,
};

/// Errors related to the preparation (instrumentation and so on) and compilation by Wasmer steps.
#[derive(Debug)]
pub enum InitializationError {
    /// Error that raises during compilation Wasm code by Wasmer.
    WasmerCreationError(String),

    /// Error that raises during creation of some Wasm objects (like table and memory) by Wasmer.
    WasmerCompileError(String),

    /// Error that raises on the preparation step.
    PrepareError(String),
}

#[derive(Debug)]
pub enum FrankError {
    /// Errors related to the preparation (instrumentation and so on) and compilation by Wasmer steps.
    InstantiationError(String),

    /// Errors for I/O errors raising while opening a file.
    IOError(String),

    /// This error type is produced by Wasmer during resolving a Wasm function.
    WasmerResolveError(String),

    /// Error related to calling a main Wasm module.
    WasmerInvokeError(String),

    /// Error indicates that smth really bad happened (like removing the global Frank state).
    FrankNotInitialized,
}

impl Error for InitializationError {}
impl Error for FrankError {}

impl std::fmt::Display for InitializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            InitializationError::WasmerCompileError(msg) => write!(f, "{}", msg),
            InitializationError::WasmerCreationError(msg) => write!(f, "{}", msg),
            InitializationError::PrepareError(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::fmt::Display for FrankError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            FrankError::InstantiationError(msg) => write!(f, "InstantiationError: {}", msg),
            FrankError::IOError(msg) => write!(f, "IOError: {}", msg),
            FrankError::WasmerResolveError(msg) => write!(f, "WasmerResolveError: {}", msg),
            FrankError::WasmerInvokeError(msg) => write!(f, "WasmerInvokeError: {}", msg),
            FrankError::FrankNotInitialized => write!(
                f,
                "Attempt to use invoke virtual machine while it hasn't been initialized.\
                 Please call the initialization method first."
            ),
        }
    }
}

impl From<CreationError> for InitializationError {
    fn from(err: CreationError) -> Self {
        InitializationError::WasmerCreationError(format!("{}", err))
    }
}

impl From<CompileError> for InitializationError {
    fn from(err: CompileError) -> Self {
        InitializationError::WasmerCompileError(format!("{}", err))
    }
}

impl From<parity_wasm::elements::Error> for InitializationError {
    fn from(err: parity_wasm::elements::Error) -> Self {
        InitializationError::PrepareError(format!("{}", err))
    }
}

impl From<InitializationError> for FrankError {
    fn from(err: InitializationError) -> Self {
        FrankError::InstantiationError(format!("{}", err))
    }
}

impl From<CallError> for FrankError {
    fn from(err: CallError) -> Self {
        match err {
            CallError::Resolve(err) => FrankError::WasmerResolveError(format!("{}", err)),
            CallError::Runtime(err) => FrankError::WasmerInvokeError(format!("{}", err)),
        }
    }
}

impl From<ResolveError> for FrankError {
    fn from(err: ResolveError) -> Self {
        FrankError::WasmerResolveError(format!("{}", err))
    }
}

impl From<RuntimeError> for FrankError {
    fn from(err: RuntimeError) -> Self {
        FrankError::WasmerInvokeError(format!("{}", err))
    }
}

impl From<WasmerError> for FrankError {
    fn from(err: WasmerError) -> Self {
        FrankError::WasmerInvokeError(format!("{}", err))
    }
}

impl From<std::io::Error> for FrankError {
    fn from(err: std::io::Error) -> Self {
        FrankError::IOError(format!("{}", err))
    }
}
