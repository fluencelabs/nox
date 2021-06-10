pub use particle_modules::Hash;
pub use particle_modules::{AddBlueprint, Dependency, ModuleRepository};

use fluence_faas::TomlFaaSNamedModuleConfig;
use host_closure::Args;
use ivalue_utils::IValue;
use serde_json::Value as JValue;

#[derive(Debug, Clone)]
pub struct RetStruct {
    pub ret_code: u32,
    pub error: String,
    pub result: String,
}

pub fn response_to_return(resp: IValue) -> RetStruct {
    match resp {
        IValue::Record(r) => {
            let ret_code = match r.get(0).unwrap() {
                IValue::U32(u) => *u,
                _ => panic!("unexpected, should be u32 ret_code"),
            };
            let msg = match r.get(1).unwrap() {
                IValue::String(u) => u.to_string(),
                _ => panic!("unexpected, should be string error message"),
            };
            if ret_code == 0 {
                RetStruct {
                    ret_code,
                    result: msg,
                    error: "".to_string(),
                }
            } else {
                RetStruct {
                    ret_code,
                    error: msg,
                    result: "".to_string(),
                }
            }
        }
        _ => panic!("unexpected, should be a record"),
    }
}

pub fn string_result(ret: RetStruct) -> Result<String, String> {
    if ret.ret_code == 0 {
        let hash: String = serde_json::from_str(&ret.result).unwrap();
        Ok(hash)
    } else {
        Err(ret.error)
    }
}

pub fn create_args(args: Vec<JValue>) -> Args {
    Args {
        service_id: "".to_string(),
        function_name: "".to_string(),
        function_args: args,
        tetraplets: vec![],
    }
}

pub fn add_module(
    repo: &ModuleRepository,
    bytes: String,
    config: TomlFaaSNamedModuleConfig,
) -> Result<String, String> {
    repo.add_module(bytes, config).map_err(|e| e.to_string())
}

pub fn add_bp(
    repo: &ModuleRepository,
    name: String,
    deps: Vec<Dependency>,
) -> Result<String, String> {
    let req = AddBlueprint {
        name,
        dependencies: deps,
    };

    repo.add_blueprint(req).map_err(|e| e.to_string())
}
