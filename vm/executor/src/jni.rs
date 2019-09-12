use jni::JNIEnv;

use crate::wasmer_executor::WasmerExecutor;
use jni::objects::{JByteBuffer, JClass, JString};
use jni::sys::{jbyteArray, jint};
use std::sync::{Arc, Mutex, RwLock};
use std::cell::RefCell;

thread_local! {
    static WASM_EXECUTOR: RefCell<Option<WasmerExecutor>> = RefCell::new(None);
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

    let executor = match WasmerExecutor::new(&file_name) {
        Ok(executor) => executor,
        Err(_) => return -1,
    };

    WASM_EXECUTOR.with(|wasm_executor| {
        *wasm_executor.borrow_mut() = Some(executor)
    });

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

    let result = WASM_EXECUTOR.with(|wasm_executor| {
        if let Some(ref mut e) = *wasm_executor.borrow_mut() {
            return e.invoke(input).unwrap()
        }
        Vec::<u8>::new()
    });

    let output: jbyteArray = env
    .new_byte_array(result.len() as i32)
    .expect("Couldn't allocate enough space for byte array");

    return output
}

// computes hash of the internal VM state
#[no_mangle]
pub extern "system" fn Java_Executor_vm_state<'a>(env: JNIEnv, class: JClass) {}
