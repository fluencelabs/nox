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
use std::cell::RefCell;

thread_local! {
    static FRANK: RefCell<Option<Frank>> = RefCell::new(None);
}

// initializes virtual machine
#[no_mangle]
pub extern "system" fn Java_fluence_vm_frank_FrankAdapter_instantiate(
    env: JNIEnv,
    _class: JClass,
    module_path: JString,
    config: JObject,
) -> jint {
    let file_name: String = env
        .get_string(module_path)
        .expect("Couldn't get module path!")
        .into();

    let config = Config::new(env, config).unwrap_or_default();

    let executor = match Frank::new(&file_name, config) {
        Ok(executor) => executor,
        Err(_) => return -1,
    };

    FRANK.with(|wasm_executor| *wasm_executor.borrow_mut() = Some(executor));

    println!("frank: init ended");

    0
}

// Invokes the main module entry point function
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

        let input = unsafe {
            Vec::<u8>::from_raw_parts(
                input.as_mut_ptr() as *mut u8,
                input.len(),
                input.capacity()
            )
        };

        let result: FrankResult = FRANK.with(|wasm_executor| {
            if let Some(ref mut e) = *wasm_executor.borrow_mut() {
                return Ok(e.invoke(&input));
            }
            Err(FrankError::FrankIncorrectState)
        })??;

        let outcome = env.byte_array_from_slice(&result.outcome).unwrap();
        let outcome = JObject::from(outcome);
        let spent_gas = JValue::from(result.spent_gas);

        let none_value =
            env.call_static_method("scala/None", "get", "()Ljava/lang/Object;", &[])?;

        env.call_static_method(
            "fluence/vm/frank/result/RawInvocationResult",
            "apply",
            "([BJ)Lfluence/vm/InvocationResult;",
            &[none_value, JValue::from(outcome), spent_gas],
        )?
        .l()
        .map_err(Into::into)
    }

    match invoke(env, fn_argument) {
        Ok(object) => object,
        Err(err) => {
            let fake_outcome = env
                .new_byte_array(0)
                .expect("jni: couldn't allocate empty byte array");
            let fake_outcome = JObject::from(fake_outcome);
            let fake_spent_gas = JValue::from(0);

            let error = env
                .new_string(format!("{}", err))
                .expect("jni: couldn't allocate new string");
            let error = JObject::from(error);
            let error = env
                .call_static_method(
                    "scala/Some",
                    "get",
                    "(Ljava/lang/Object;)Lscala/Some;",
                    &[JValue::from(error)],
                )
                .expect("jni: couldn't allocate a Some object");

            env.call_static_method(
                "fluence/vm/frank/result/RawInvocationResult",
                "apply",
                "(Lscala/Option;[BJ)Lfluence/vm/frank/result/RawInvocationResult;",
                &[
                    error,
                    JValue::from(fake_outcome),
                    JValue::from(fake_spent_gas),
                ],
            )
            .expect("jni: couldn't allocate RawInvocationResult object")
            .l()
            .expect("jni: couldn't convert RawInvocationResult to Java Object")
        }
    }
}

// computes hash of the internal VM state
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
