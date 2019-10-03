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

/// Defines export functions that will be accessible from the Scala part.

use crate::config::Config;
use crate::errors::FrankError;
use crate::frank::{Frank, FRANK};
use crate::frank_result::FrankResult;
use crate::jni::jni_results::*;
use jni::objects::{JClass, JObject, JString};
use jni::sys::jbyteArray;
use jni::JNIEnv;
use sha2::digest::generic_array::GenericArray;

/// Initializes Frank virtual machine.
#[no_mangle]
pub extern "system" fn Java_fluence_vm_frank_FrankAdapter_initialize<'a>(
    env: JNIEnv<'a>,
    _class: JClass,
    module_path: JString,
    config: JObject,
) -> JObject<'a> {
    fn initialize<'a>(
        env: &JNIEnv<'a>,
        module_path: JString,
        config: JObject,
    ) -> Result<(bool), FrankError> {
        let file_name: String = env.get_string(module_path)?.into();
        let config = Config::new(&env, config)?;
        let frank = Frank::new(&file_name, config)?;

        unsafe { FRANK = Some(Box::new(frank.0)) };

        Ok(frank.1)
    }

    match initialize(&env, module_path, config) {
        Ok(expects_eths) => create_initialization_result(&env, None, expects_eths),
        Err(err) => create_initialization_result(&env, Some(format!("{}", err)), false),
    }
}

/// Invokes the main module entry point function.
#[no_mangle]
pub extern "system" fn Java_fluence_vm_frank_FrankAdapter_invoke<'a>(
    env: JNIEnv<'a>,
    _class: JClass,
    fn_argument: jbyteArray,
) -> JObject<'a> {
    fn invoke(env: &JNIEnv, fn_argument: jbyteArray) -> Result<FrankResult, FrankError> {
        let input_len = env.get_array_length(fn_argument)?;
        let mut input = vec![0; input_len as _];
        env.get_byte_array_region(fn_argument, 0, input.as_mut_slice())?;

        // converts Vec<i8> to Vec<u8> without additional allocation
        let u8_input = unsafe {
            Vec::<u8>::from_raw_parts(input.as_mut_ptr() as *mut u8, input.len(), input.capacity())
        };
        std::mem::forget(input);

        unsafe {
            match FRANK {
                Some(ref mut vm) => Ok(vm.invoke(&u8_input)?),
                None => Err(FrankError::FrankNotInitialized),
            }
        }
    }

    match invoke(&env, fn_argument) {
        Ok(result) => create_invocation_result(&env, None, result),
        Err(err) => {
            create_invocation_result(&env, Some(format!("{}", err)), FrankResult::default())
        }
    }
}

/// Computes hash of the internal VM state.
#[no_mangle]
pub extern "system" fn Java_fluence_vm_frank_FrankAdapter_computeVmState<'a>(
    env: JNIEnv<'a>,
    _class: JClass,
) -> JObject<'a> {
    unsafe {
        match FRANK {
            Some(ref mut vm) => {
                let state = vm.compute_vm_state_hash();
                create_state_computation_result(&env, None, state)
            }
            None => create_state_computation_result(
                &env,
                Some(format!("{}", FrankError::FrankNotInitialized)),
                GenericArray::default(),
            ),
        }
    }
}
