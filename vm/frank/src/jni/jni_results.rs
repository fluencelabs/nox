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

use crate::frank_result::FrankResult;
use crate::jni::option::*;
use jni::objects::{JObject, JValue};
use jni::JNIEnv;
use sha2::{digest::generic_array::GenericArray, digest::FixedOutput, Sha256};

/// Creates RawInitializationResult object for Scala part.
pub fn create_initialization_result<'a>(env: &JNIEnv<'a>, error: Option<String>) -> JObject<'a> {
    let error_value = match error {
        Some(err) => create_some_value(&env, err),
        None => create_none_value(&env),
    };

    env.call_static_method(
        "fluence/vm/frank/result/RawInitializationResult",
        "apply",
        "(Lscala/Option;)Lfluence/vm/frank/result/RawInitializationResult;",
        &[error_value],
    )
    .expect("jni: couldn't allocate RawInitializationResult object")
    .l()
    .expect("jni: couldn't convert RawInitializationResult to Java Object")
}

/// Creates RawInvocationResult object for Scala part.
pub fn create_invocation_result<'a>(
    env: &JNIEnv<'a>,
    error: Option<String>,
    result: FrankResult,
) -> JObject<'a> {
    let error_value = match error {
        Some(err) => create_some_value(&env, err),
        None => create_none_value(&env),
    };

    let outcome = env.byte_array_from_slice(&result.outcome).unwrap();
    let outcome = JObject::from(outcome);
    let spent_gas = JValue::from(result.spent_gas);

    env.call_static_method(
        "fluence/vm/frank/result/RawInvocationResult",
        "apply",
        "(Lscala/Option;[BJ)Lfluence/vm/frank/result/RawInvocationResult;",
        &[error_value, JValue::from(outcome), spent_gas],
    )
    .expect("jni: couldn't allocate RawInvocationResult object")
    .l()
    .expect("jni: couldn't convert RawInvocationResult to Java Object")
}

/// Creates RawStateComputationResult object for Scala part.
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

    env.call_static_method(
        "fluence/vm/frank/result/RawStateComputationResult",
        "apply",
        "(Lscala/Option;[B)Lfluence/vm/frank/result/RawStateComputationResult;",
        &[error_value, JValue::from(state)],
    )
    .expect("jni: couldn't allocate RawInvocationResult object")
    .l()
    .expect("jni: couldn't convert RawInvocationResult to Java Object")
}
