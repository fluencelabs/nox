use chain_data::FunctionTrait;
use ethabi::{Function, ParamType, StateMutability};

/// @dev Returns epoch duration
/// @return epochDuration in seconds
/// function epochDuration() external view returns (uint256);
pub struct EpochDurationFunction;

impl FunctionTrait for EpochDurationFunction {
    fn function() -> Function {
        #[allow(deprecated)]
        Function {
            name: "epochDuration".to_string(),
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
