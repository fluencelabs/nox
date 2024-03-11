use chain_data::ChainFunction;
use ethabi::{Function, ParamType, StateMutability};

/// @dev Returns current epoch
/// @return current epoch number
/// function currentEpoch() external view returns (uint256);
pub struct CurrentEpochFunction;
impl ChainFunction for CurrentEpochFunction {
    fn function() -> Function {
        #[allow(deprecated)]
        Function {
            name: "currentEpoch".to_string(),
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
