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

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Count of Wasm memory pages that will be preallocated on the VM startup.
    /// Each Wasm pages is 65536 bytes long.
    pub mem_pages_count: i32,

    /// If true, registers the logger Wasm module with name 'logger'.
    /// This functionality is just for debugging, and this module will be disabled in future.
    pub logger_enabled: bool,

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
        }
    }
}
