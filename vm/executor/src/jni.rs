use jni::JNIEnv;

use jni::objects::{JByteBuffer, JClass, JString};
use jni::sys::{jbyteArray, jint};
use crate::wasmer_executor::WasmerExecutor;

use std::sync::Mutex;

lazy_static! {
    static ref WASM_EXECUTOR: Mutex<Option<WasmerExecutor>> = None;
}

// initializes virtual machine
#[no_mangle]
pub extern "system" fn Java_Executor_init(
    env: JNIEnv,
    class: JClass,
    module_path: JString,
) -> jint {
    let file_name: String = env
        .get_string(module_path)
        .expect("Couldn't get module path!")
        .into();

    let mut executor = WASM_EXECUTOR.lock().unwrap();

    executor = match WasmerExecutor::new(&file_name) {
        Ok(executor) => Some(executor),
        Err(_) => return -1
    };

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

    let mut executor = WASM_EXECUTOR.lock().unwrap();

    let result = match executor {
        None => {
            env
                .new_byte_array(0)
                .expect("Couldn't allocate enough space for byte array");
        },
        Some(executor) => executor.invoke(input),
    };

    let output: jbyteArray = env
        .new_byte_array(result.len())
        .expect("Couldn't allocate enough space for byte array");

    output
}

// computes hash of the internal VM state
#[no_mangle]
pub extern "system" fn Java_Executor_vm_state<'a>(
    env: JNIEnv,
    class: JClass,
) {}
