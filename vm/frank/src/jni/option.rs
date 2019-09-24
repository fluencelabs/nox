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

/// Defines functions to construct the Scala Option[String] objects by calling the apply method:
///
/// ```
/// public static Option apply(final Object x) {
///    return Option$.MODULE$.apply(var0);
/// }
/// ```

use jni::JNIEnv;
use jni::objects::{JObject, JValue};

/// creates Scala None value
pub fn create_none_value<'a>(env: &JNIEnv<'a>) -> JValue<'a> {
    env.call_static_method(
        "scala/Option",
        "apply",
        "(Ljava/lang/Object;)Lscala/Option;",
        &[JValue::from(JObject::null())],
    )
    .map_err(|err| format!("{}", err))
    .expect("jni: error while creating None object")
}

/// creates Scala Some[String] value
pub fn create_some_value<'a>(env: &JNIEnv<'a>, value: String) -> JValue<'a> {
    let value = env
        .new_string(value)
        .expect("jni: couldn't allocate new string");

    let value = JObject::from(value);
    env.call_static_method(
        "scala/Option",
        "apply",
        "(Ljava/lang/Object;)Lscala/Option;",
        &[JValue::from(value)],
    )
    .expect("jni: couldn't allocate a Some object")
}
