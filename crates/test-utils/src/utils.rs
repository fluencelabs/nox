use ivalue_utils::IValue;

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
