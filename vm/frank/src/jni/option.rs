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

use jni::objects::{JObject, JValue};

/// creates Scala None value
pub fn create_none_value(env: JNIEnv) -> JValue {
    env.
        call_static_method("scala/None", "get", "()Ljava/lang/Object;", &[])
        .expect("jni: error while creating None object")
}

/// creates Scala Some[String] value
pub fn create_some_value(env: JNIEnv, value: String) -> JValue {
    let value = env_copy
        .new_string(value)
        .expect("jni: couldn't allocate new string");

    let value = JObject::from(error);
    env_copy
        .call_static_method(
            "scala/Some",
            "get",
            "(Ljava/lang/Object;)Lscala/Some;",
            &[JValue::from(error)],
        )
        .expect("jni: couldn't allocate a Some object")
}
