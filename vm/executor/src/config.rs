use jni::JNIEnv;
use jni::objects::JClass;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Count of Wasm memory pages that will be preallocated on the VM startup.
    /// Each Wasm pages is 65536 bytes log.
    pub memory_pages_count: i32,

    /// The name of the main module handler function.
    pub invoke_function_name: String,

    /// The name of function that should be called for allocation memory. This function
    /// is used for passing array of bytes to the main module.
    pub allocate_function_name: String,

    /// The name of function that should be called for deallocation of
    /// previously allocated memory by allocateFunction.
    pub deallocate_function_name: String,

    /// If true, registers the logger Wasm module as 'logger'
    /// This functionality is just for debugging, and this module will be disabled in future
    logger_module_enabled: bool,

    /// The memory will be split by chunks to be able to build Merkle Tree on top of it.
    /// Size of memory in bytes must be dividable by chunkSize.
    chunk_size: i32,
}

impl Default for Config {
    fn default() -> Self {
        // some reasonable defaults
        Self {
            // 65536*1600 ~ 100 Mb
            memory_pages_count: 1600,
            invoke_function_name: "invoke".to_string(),
            allocate_function_name: "allocate".to_string(),
            deallocate_function_name: "deallocate".to_string(),
            logger_module_enabled: true,
            chunk_size: 4096,
        }
    }
}

/*
  memPages: 64

  loggerEnabled: true

  chunkSize: 4096

  mainModuleConfig: {
    allocateFunctionName: "allocate"

    deallocateFunctionName: "deallocate"

    invokeFunctionName: "invoke"
  }
*/
const CONFIG_CLASS_NAME: &str = "Config";

impl Config {
    // creates new config based on the supplied Java object Config
    pub fn new(env: JNIEnv, config: &JClass) -> std::result::Result<Self, ()> {

    }
}
