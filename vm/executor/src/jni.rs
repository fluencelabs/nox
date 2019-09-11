use jni::JNIEnv;

use jni::objects::{JByteBuffer, JClass, JString};
use jni::sys::{jbyteArray, jint, jstring};

// initializes virtual machine
#[no_mangle]
pub extern "system" fn Java_Executor_init(
    env: JNIEnv,
    class: JClass,
    module_path: JString,
) -> jint {
    let input: String = env
        .get_string(module_path)
        .expect("Couldn't get module path!")
        .into();

    println!("{}", input);

    let output = env
        .new_string(format!("Hello, {}!", input))
        .expect("Couldn't create java string!");

    0
}

// Invokes the main module entry point function
#[no_mangle]
pub extern "system" fn Java_Executor_invoke<'a>(
    env: JNIEnv,
    class: JClass,
    fn_argument: JByteBuffer,
) -> jbyteArray {
    let input = env
        .get_direct_buffer_address(fn_argument)
        .expect("Couldn't get function argument value");

    let output: jbyteArray = env
        .new_byte_array(10)
        .expect("Couldn't allocate enough space for byte array");

    output
}
