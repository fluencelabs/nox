/*
 * Copyright 2019 Fluence Labs Limited
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

use jni::errors::Error as JNIWrapperError;
use wasmer_runtime::error::{
    CallError, CompileError, CreationError, Error, ResolveError, RuntimeError,
};

// TODO: more errors to come (when preparation step will be landed)
/// Errors related to the preparation (instrumentation and so on) and compilation by Wasmer steps.
pub enum InstantiationError {
    /// Error that raises during compilation Wasm code by Wasmer.
    WasmerCreationError(String),

    /// Error that raises during creation of some Wasm objects (like table and memory) by Wasmer.
    WasmerCompileError(String),
}

pub enum FrankError {
    /// Errors related to the preparation (instrumentation and so on) and compilation by Wasmer steps.
    InstantiationError(String),

    /// Errors related to parameter passing from Java to Rust and back.
    JNIError(String),

    /// Errors for I/O errors raising while opening a file.
    IOError(String),

    /// This error type is produced by Wasmer during resolving a Wasm function.
    WasmerResolveError(String),

    /// Error related to calling a main Wasm module.
    WasmerInvokeError(String),

    /// Error indicates that smth really bad happened (like removing the global Frank state).
    FrankNotInitialized,
}

impl std::fmt::Display for InstantiationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            InstantiationError::WasmerCompileError(msg) => write!(f, "{}", msg),
            InstantiationError::WasmerCreationError(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::fmt::Display for FrankError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            FrankError::InstantiationError(msg) => write!(f, "InstantiationError: {}", msg),
            FrankError::JNIError(msg) => write!(f, "JNIError: {}", msg),
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

impl From<JNIWrapperError> for FrankError {
    fn from(err: JNIWrapperError) -> Self {
        FrankError::JNIError(format!("{}", err))
    }
}

impl From<CreationError> for InstantiationError {
    fn from(err: CreationError) -> Self {
        InstantiationError::WasmerCreationError(format!("{}", err))
    }
}

impl From<CompileError> for InstantiationError {
    fn from(err: CompileError) -> Self {
        InstantiationError::WasmerCompileError(format!("{}", err))
    }
}

impl From<InstantiationError> for FrankError {
    fn from(err: InstantiationError) -> Self {
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

impl From<Error> for FrankError {
    fn from(err: Error) -> Self {
        FrankError::WasmerInvokeError(format!("{}", err))
    }
}

impl From<std::io::Error> for FrankError {
    fn from(err: std::io::Error) -> Self {
        FrankError::IOError(format!("{}", err))
    }
}
