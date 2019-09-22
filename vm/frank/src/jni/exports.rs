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
use crate::errors::FrankError;
use crate::frank::Frank;
use crate::frank_result::FrankResult;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::{jbyteArray, jint};
use sha2::digest::generic_array::GenericArray;
use crate::jni::jni_results::*;

static mut FRANK: Option<Frank> = None;

/// Initializes Frank virtual machine.
#[no_mangle]
pub extern "system" fn Java_fluence_vm_frank_FrankAdapter_initialize<'a>(
    env: JNIEnv<'a>,
    _class: JClass,
    module_path: JString,
    config: JObject,
) -> JObject<'a> {
    fn initialize<'a>(env: JNIEnv<'a>, module_path: JString, config: JObject) -> Result<(), FrankError> {
        let file_name: String = env.get_string(module_path)?.into();
        let mut config = Config::new(env, config)?;
        let executor = Frank::new(&file_name, config)?;
        unsafe { FRANK = Some(executor) };

        println!("frank: init ended");
        Ok(())
    }

    let env_copy = env.clone();
    match initialize(env, module_path, config) {
        Ok(_) => create_initialization_result(env, None),
        Err(err) => create_initialization_result(env, Some(format!("{}", err)))
    }
}

/// Invokes the main module entry point function.
#[no_mangle]
pub extern "system" fn Java_fluence_vm_frank_FrankAdapter_invoke<'a>(
    env: JNIEnv<'a>,
    _class: JClass,
    fn_argument: jbyteArray,
) -> JObject<'a> {
    fn invoke<'a>(env: JNIEnv<'a>, fn_argument: jbyteArray) -> Result<JObject<'a>, FrankError> {
        let input_len = env.get_array_length(fn_argument)?;
        let mut input = vec![0; input_len as _];
        env.get_byte_array_region(fn_argument, 0, input.as_mut_slice())?;

        // converts Vec<i8> to Vec<u8>
        let input = unsafe {
            Vec::<u8>::from_raw_parts(input.as_mut_ptr() as *mut u8, input.len(), input.capacity())
        };

        unsafe {
            match FRANK {
                Some(ref mut vm) => vm.invoke(&input)?,
                None =>  Err(FrankError::FrankIncorrectState)

            }
        }?
    }

    let env_copy = env.clone();
    match invoke(env, fn_argument) {
        Ok(object) => object,
        Err(err) => {
        }
    }
}

/// Computes hash of the internal VM state.
#[no_mangle]
pub extern "system" fn Java_fluence_vm_frank_FrankAdapter_getVmState(
    env: JNIEnv,
    _class: JClass,
) -> jbyteArray {
    let result = FRANK.with(|wasm_executor| {
        if let Some(ref mut e) = *wasm_executor.borrow_mut() {
            return e.compute_vm_state_hash();
        }
        println!("!!");
        GenericArray::default()
    });

    env.byte_array_from_slice(result.as_slice())
        .expect("Couldn't allocate enough space for byte array")
}
