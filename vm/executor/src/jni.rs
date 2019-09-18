use jni::JNIEnv;

use crate::wasmer_executor::WasmerExecutor;
use jni::objects::{JByteBuffer, JClass, JString, JObject};
use jni::sys::{jbyteArray, jint};
use std::sync::{Arc, Mutex, RwLock};
use std::cell::RefCell;

thread_local! {
    static WASM_EXECUTOR: RefCell<Option<WasmerExecutor>> = RefCell::new(None);
}

// initializes virtual machine
#[no_mangle]
pub extern "system" fn Java_fluence_vm_wasmer_WasmerConnector_init(
    env: JNIEnv,
    class: JClass,
    module_path: JString,
    config: JObject,
) -> jint {
    println!("wasm executor: init started");

    let file_name: String = env
        .get_string(module_path)
        .expect("Couldn't get module path!")
        .into();

    let config = env.find_class(CONFIG_CLASS_NAME).unwrap();
    let tt = env.get_field(config, "")

    let executor = match WasmerExecutor::new(&file_name) {
        Ok(executor) => executor,
        Err(_) => return -1,
    };

    WASM_EXECUTOR.with(|wasm_executor| {
        *wasm_executor.borrow_mut() = Some(executor)
    });

    println!("wasm executor: init ended");

    0
}

// Invokes the main module entry point function
#[no_mangle]
pub extern "system" fn Java_fluence_vm_wasmer_WasmerConnector_invoke<'a>(
    env: JNIEnv,
    class: JClass,
    fn_argument: jbyteArray,
) -> jbyteArray {
    let input_len = env.get_array_length(fn_argument).unwrap();
    println!("argument length is {}", input_len);

    let mut input = Vec::<i8>::with_capacity(input_len as _);
    input.resize(input_len as _, 0);

    env.get_byte_array_region(fn_argument, 0, input.as_mut_slice())
        .expect("Couldn't get function argument value");

    let result = WASM_EXECUTOR.with(|wasm_executor| {
        if let Some(ref mut e) = *wasm_executor.borrow_mut() {
            return e.invoke(&input).unwrap()
        }
        Vec::<u8>::new()
    });

    let output: jbyteArray = env
        .byte_array_from_slice(&result)
        .expect("Couldn't allocate enough space for byte array");

    return output
}

// computes hash of the internal VM state
#[no_mangle]
pub extern "system" fn Java_fluence_vm_wasmer_WasmerConnector_getVmState<'a>(env: JNIEnv, class: JClass) -> jbyteArray {
    let output: jbyteArray = env
        .new_byte_array(1)
        .expect("Couldn't allocate enough space for byte array");

    return output
}
