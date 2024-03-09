use chain_data::ChainFunction;
use ethabi::{Function, ParamType, StateMutability};

/// @dev Returns epoch duration
/// @return epochDuration in seconds
/// function epochDuration() external view returns (uint256);
pub struct EpochDurationFunction;

impl ChainFunction for EpochDurationFunction {
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

    fn result_signature() -> Vec<ParamType> {
        vec![ParamType::Uint(256)]
    }
}
