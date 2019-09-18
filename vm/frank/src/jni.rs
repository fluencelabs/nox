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

use jni::JNIEnv;

use crate::config::Config;
use crate::frank::Frank;
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jbyteArray, jint};
use sha2::digest::generic_array::GenericArray;
use sha2::digest::DynDigest;
use std::cell::RefCell;

thread_local! {
    static FRANK: RefCell<Option<Frank>> = RefCell::new(None);
}

// initializes virtual machine
#[no_mangle]
pub extern "system" fn Java_fluence_vm_wasmer_WasmerConnector_instantiate(
    env: JNIEnv,
    _class: JClass,
    module_path: JString,
    config: JObject,
) -> jint {
    println!("wasm executor: init started");

    let file_name: String = env
        .get_string(module_path)
        .expect("Couldn't get module path!")
        .into();

    let config = Config::new(env, config).unwrap();

    let executor = match Frank::new(&file_name, config) {
        Ok(executor) => executor,
        Err(_) => return -1,
    };

    FRANK.with(|wasm_executor| *wasm_executor.borrow_mut() = Some(executor));

    println!("wasm executor: init ended");

    0
}

// Invokes the main module entry point function
#[no_mangle]
pub extern "system" fn Java_fluence_vm_wasmer_WasmerConnector_invoke(
    env: JNIEnv,
    _class: JClass,
    fn_argument: jbyteArray,
) -> jbyteArray {
    let input_len = env.get_array_length(fn_argument).unwrap();
    println!("wasm executor: argument length is {}", input_len);

    let mut input = vec![0; input_len as _];

    env.get_byte_array_region(fn_argument, 0, input.as_mut_slice())
        .expect("Couldn't get function argument value");

    let result = FRANK.with(|wasm_executor| {
        if let Some(ref mut e) = *wasm_executor.borrow_mut() {
            return e.invoke(&input).unwrap();
        }
        Vec::<u8>::new()
    });

    env.byte_array_from_slice(&result)
        .expect("Couldn't allocate enough space for byte array")
}

// computes hash of the internal VM state
#[no_mangle]
pub extern "system" fn Java_fluence_vm_wasmer_WasmerConnector_getVmState(
    env: JNIEnv,
    _class: JClass,
) -> jbyteArray {
    let result = FRANK.with(|wasm_executor| {
        if let Some(ref mut e) = *wasm_executor.borrow_mut() {
            return e.compute_vm_state_hash();
        }
        GenericArray::default()
    });

    env.byte_array_from_slice(result.as_slice())
        .expect("Couldn't allocate enough space for byte array")
}
