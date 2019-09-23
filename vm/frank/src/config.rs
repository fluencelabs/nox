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

/// Defines Config struct that is similar to vm/src/main/scala/fluence/vm/config/VmConfig.scala.

use crate::errors::FrankError;
use jni::objects::{JObject, JString};
use jni::JNIEnv;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Count of Wasm memory pages that will be preallocated on the VM startup.
    /// Each Wasm pages is 65536 bytes long.
    pub mem_pages_count: i32,

    /// If true, registers the logger Wasm module with name 'logger'.
    /// This functionality is just for debugging, and this module will be disabled in future.
    pub logger_enabled: bool,

    /// Memory will be split by chunks to be able to build Merkle Tree on top of it.
    /// Size of memory in bytes must be dividable by chunkSize.
    pub chunk_size: i32,

    /// The name of the main module handler function.
    pub invoke_function_name: String,

    /// The name of function that should be called for allocation memory. This function
    /// is used for passing array of bytes to the main module.
    pub allocate_function_name: String,

    /// The name of function that should be called for deallocation of
    /// previously allocated memory by allocateFunction.
    pub deallocate_function_name: String,
}

impl Default for Config {
    fn default() -> Self {
        // some reasonable defaults
        Self {
            // 65536*1600 ~ 100 Mb
            mem_pages_count: 1600,
            invoke_function_name: "invoke".to_string(),
            allocate_function_name: "allocate".to_string(),
            deallocate_function_name: "deallocate".to_string(),
            logger_enabled: true,
            chunk_size: 4096,
        }
    }
}

impl Config {
    /// Creates a new config based on the supplied Scala object Config.
    /// This config should have the following structure:
    ///
    /// ```
    /// case class MainModuleConfig(
    ///   name: Option[String],
    ///   allocateFunctionName: String,
    ///   deallocateFunctionName: String,
    ///   invokeFunctionName: String
    /// )
    ///
    /// case class VmConfig(
    ///   memPagesCount: Int,
    ///   loggerEnabled: Boolean,
    ///   chunkSize: Int,
    ///   mainModuleConfig: MainModuleConfig
    /// )
    /// ```
    ///
    pub fn new(env: JNIEnv, config: JObject) -> Result<Box<Self>, FrankError> {
        let mem_pages_count = env.call_method(config, "memPagesCount", "()I", &[])?.i()?;
        let logger_enabled = env.call_method(config, "loggerEnabled", "()Z", &[])?.z()?;
        let chunk_size = env.call_method(config, "chunkSize", "()I", &[])?.i()?;

        let main_module_config = env
            .call_method(
                config,
                "mainModuleConfig",
                "()Lfluence/vm/config/MainModuleConfig;",
                &[],
            )?
            .l()?;

        let allocate_function_name = env
            .call_method(
                main_module_config,
                "allocateFunctionName",
                "()Ljava/lang/String;",
                &[],
            )?
            .l()?;
        let deallocate_function_name = env
            .call_method(
                main_module_config,
                "deallocateFunctionName",
                "()Ljava/lang/String;",
                &[],
            )?
            .l()?;
        let invoke_function_name = env
            .call_method(
                main_module_config,
                "invokeFunctionName",
                "()Ljava/lang/String;",
                &[],
            )?
            .l()?;

        // converts JObject to JString (without copying, just enum type changes)
        let allocate_function_name = env.get_string(JString::from(allocate_function_name))?;
        let deallocate_function_name = env.get_string(JString::from(deallocate_function_name))?;
        let invoke_function_name = env.get_string(JString::from(invoke_function_name))?;

        Ok(Box::new(Self {
            mem_pages_count,
            logger_enabled,
            chunk_size,
            // and then finally to Rust String (requires one copy)
            invoke_function_name: String::from(invoke_function_name),
            allocate_function_name: String::from(allocate_function_name),
            deallocate_function_name: String::from(deallocate_function_name),
        }))
    }
}
