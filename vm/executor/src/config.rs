use jni::objects::{JObject, JString};
use jni::JNIEnv;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Count of Wasm memory pages that will be preallocated on the VM startup.
    /// Each Wasm pages is 65536 bytes log.
    pub mem_pages_count: i32,

    /// If true, registers the logger Wasm module as 'logger'
    /// This functionality is just for debugging, and this module will be disabled in future
    pub logger_enabled: bool,

    /// The memory will be split by chunks to be able to build Merkle Tree on top of it.
    /// Size of memory in bytes must be dividable by chunkSize.
    pub chunk_size: i32,

    /// The name of the main module handler function.
    pub invoke_function_name: String,

    /// The name of function that should be called for allocation memory. This function
    /// is used for passing array of bytes to the main module.
    pub allocate_function_name: String,

    /// The name of function that should be called for deallocation of
    /// previously allocated memory by allocateFunction.
    pub deallocate_function_name: String,
}

impl Default for Config {
    fn default() -> Self {
        // some reasonable defaults
        Self {
            // 65536*1600 ~ 100 Mb
            mem_pages_count: 1600,
            invoke_function_name: "invoke".to_string(),
            allocate_function_name: "allocate".to_string(),
            deallocate_function_name: "deallocate".to_string(),
            logger_enabled: true,
            chunk_size: 4096,
        }
    }
}

const CONFIG_CLASS_NAME: &str = "Config";

impl Config {
    // creates new config based on the supplied Java object Config
    pub fn new(env: JNIEnv, config: JObject) -> std::result::Result<Self, ()> {
        let mem_pages_count = env
            .get_field(config, "memPagesCount", "I")
            .unwrap()
            .i()
            .unwrap();
        let logger_enabled = env
            .get_field(config, "loggerEnabled", "Z")
            .unwrap()
            .z()
            .unwrap();
        let chunk_size = env
            .get_field(config, "chunkSize", "I")
            .unwrap()
            .i()
            .unwrap();

        let allocate_function_name = env
            .get_field(config, "allocateFunctionName", "java/lang/String")
            .unwrap()
            .l()
            .unwrap();
        let deallocate_function_name = env
            .get_field(config, "deallocateFunctionName", "java/lang/String")
            .unwrap()
            .l()
            .unwrap();
        let invoke_function_name = env
            .get_field(config, "invokeFunctionName", "java/lang/String")
            .unwrap()
            .l()
            .unwrap();

        Ok(Self {
            mem_pages_count,
            logger_enabled,
            chunk_size,
            invoke_function_name: String::from(invoke_function_name),
            allocate_function_name: String::from(allocate_function_name),
            deallocate_function_name: String::from(deallocate_function_name),
        })
    }
}
