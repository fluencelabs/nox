use jni::JNIEnv;

use jni::objects::{JClass, JString};

use jni::sys::jstring;

// initializes virtual machine
#[no_mangle]
pub extern "system" fn Java_Executor_init(
    env: JNIEnv,
    class: JClass,
    module_path: JString)
    -> jstring {

    let input: String =
        env.get_string(module_path).expect("Couldn't get module path!").into();

    let output = env.new_string(format!("Hello, {}!", input))
        .expect("Couldn't create java string!");

    output.into_inner()
}
