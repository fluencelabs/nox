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

/// Defines functions used to construct result of the VM invocation for the Scala part.
/// Corresponding case classes could be found in vm/src/main/scala/fluence/vm/frank/result.
use crate::frank_result::FrankResult;
use crate::jni::option::*;
use jni::objects::{JObject, JValue};
use jni::JNIEnv;
use sha2::{digest::generic_array::GenericArray, digest::FixedOutput, Sha256};

/// Creates RawInitializationResult object for the Scala part.
pub fn create_initialization_result<'a>(
    env: &JNIEnv<'a>,
    error: Option<String>,
    expects_eths: bool,
) -> JObject<'a> {
    let error_value = match error {
        Some(err) => create_some_value(&env, err),
        None => create_none_value(&env),
    };

    // public static RawInitializationResult apply(final Option error, final Boolean expects_eth) {
    //   return RawInitializationResult$.MODULE$.apply(var0);
    // }
    env.call_static_method(
        "fluence/vm/frank/result/RawInitializationResult",
        "apply",
        "(Lscala/Option;Z)Lfluence/vm/frank/result/RawInitializationResult;",
        &[error_value, JValue::from(expects_eths)],
    )
    .expect("jni: couldn't allocate a new RawInitializationResult object")
    .l()
    .expect("jni: couldn't convert RawInitializationResult to a Java Object")
}

/// Creates RawInvocationResult object for the Scala part.
pub fn create_invocation_result<'a>(
    env: &JNIEnv<'a>,
    error: Option<String>,
    result: FrankResult,
) -> JObject<'a> {
    let error_value = match error {
        Some(err) => create_some_value(&env, err),
        None => create_none_value(&env),
    };

    // TODO: here we have 2 copying of result, first is from Wasm memory to a Vec<u8>, second is
    // from the Vec<u8> to Java byte array. Optimization might be possible after memory refactoring.
    let outcome = env.byte_array_from_slice(&result.outcome).unwrap();
    let outcome = JObject::from(outcome);
    let spent_gas = JValue::from(result.spent_gas);

    // public static RawInvocationResult apply(final Option error, final byte[] output, final long spentGas) {
    //   return RawInvocationResult$.MODULE$.apply(var0, var1, var2);
    // }
    env.call_static_method(
        "fluence/vm/frank/result/RawInvocationResult",
        "apply",
        "(Lscala/Option;[BJ)Lfluence/vm/frank/result/RawInvocationResult;",
        &[error_value, JValue::from(outcome), spent_gas],
    )
    .expect("jni: couldn't allocate a new RawInvocationResult object")
    .l()
    .expect("jni: couldn't convert RawInvocationResult to a Java Object")
}

/// Creates RawStateComputationResult object for the Scala part.
pub fn create_state_computation_result<'a>(
    env: &JNIEnv<'a>,
    error: Option<String>,
    state: GenericArray<u8, <Sha256 as FixedOutput>::OutputSize>,
) -> JObject<'a> {
    let error_value = match error {
        Some(err) => create_some_value(&env, err),
        None => create_none_value(&env),
    };

    let state = env
        .byte_array_from_slice(state.as_slice())
        .expect("jni: couldn't allocate enough space for byte array");
    let state = JObject::from(state);

    // public static RawStateComputationResult apply(final Option error, final byte[] state) {
    //   return RawStateComputationResult$.MODULE$.apply(var0, var1);
    // }
    env.call_static_method(
        "fluence/vm/frank/result/RawStateComputationResult",
        "apply",
        "(Lscala/Option;[B)Lfluence/vm/frank/result/RawStateComputationResult;",
        &[error_value, JValue::from(state)],
    )
    .expect("jni: couldn't allocate a new RawInvocationResult object")
    .l()
    .expect("jni: couldn't convert RawInvocationResult to a Java Object")
}
