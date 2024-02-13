use chain_data::FunctionTrait;
use ethabi::{Function, ParamType, StateMutability};

/// function getGlobalNonce() external view returns (bytes32);

pub struct GetGlobalNonceFunction;

impl FunctionTrait for GetGlobalNonceFunction {
    fn function() -> Function {
        #[allow(deprecated)]
        Function {
            name: "getGlobalNonce".to_string(),
            inputs: vec![],
            outputs: vec![],
            constant: None,
            state_mutability: StateMutability::View,
        }
    }

    fn signature() -> Vec<ParamType> {
        vec![ParamType::FixedBytes(32)]
    }
}
