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

use crate::jni::option::*;
use jni::JNIEnv;
use jni::objects::{JObject, JValue};
use crate::frank_result::FrankResult;

/// Creates RawInitializationResult object.
pub fn create_initialization_result(env: JNIEnv, error: Option<String>) -> JObject {
    let env_clone = env.clone();

    let error_value = match error {
        Some(err) => create_some_value( env_clone, err),
        None => create_none_value( env_clone),
    };

    env.call_static_method(
        "fluence/vm/frank/result/RawInitializationResult",
        "apply",
        "(Lscala/Option;)Lfluence/vm/RawInitializationResult;",
        &[error_value],
    )
        .expect("jni: couldn't allocate RawInitializationResult object")
        .l()
        .expect("jni: couldn't convert RawInitializationResult to Java Object")
}

/// Creates RawInvocationResult object.
pub fn create_invocation_result(env: JNIEnv, error: Option<String>, result: FrankResult) -> JObject {
    let env_clone = env.clone();
    let error_value = match error {
        Some(err) => create_some_value(env_clone, err),
        None => create_none_value(env_clone),
    };

    let outcome = env.byte_array_from_slice(&result.outcome).unwrap();
    let outcome = JObject::from(outcome);
    let spent_gas = JValue::from(result.spent_gas);

    env.call_static_method(
        "fluence/vm/frank/result/RawInvocationResult",
        "apply",
        "(Lscala/Option;[BJ)Lfluence/vm/RawInvocationResult;",
        &[error_value, JValue::from(outcome), spent_gas],
    ) .expect("jni: couldn't allocate RawInitializationResult object")
        .l()
        .expect("jni: couldn't convert RawInitializationResult to Java Object")
}
