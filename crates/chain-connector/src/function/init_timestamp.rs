use chain_data::FunctionTrait;
use ethabi::{Function, ParamType, StateMutability};

/// @dev Returns epoch init timestamp
/// @return initTimestamp in seconds
/// function initTimestamp() external view returns (uint256);
///

pub struct InitTimestampFunction;

impl FunctionTrait for InitTimestampFunction {
    fn function() -> Function {
        #[allow(deprecated)]
        Function {
            name: "initTimestamp".to_string(),
            inputs: vec![],
            outputs: vec![],
            constant: None,
            state_mutability: StateMutability::View,
        }
    }

    fn signature() -> Vec<ParamType> {
        vec![ParamType::Uint(256)]
    }
}
