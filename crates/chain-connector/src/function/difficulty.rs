use chain_data::ChainFunction;
use ethabi::{Function, ParamType, StateMutability};

/// function difficulty() external view returns (bytes32);
pub struct DifficultyFunction;
impl ChainFunction for DifficultyFunction {
    fn function() -> Function {
        #[allow(deprecated)]
        Function {
            name: "difficulty".to_string(),
            inputs: vec![],
            outputs: vec![],
            constant: None,
            state_mutability: StateMutability::View,
        }
    }

    fn result_signature() -> Vec<ParamType> {
        vec![ParamType::FixedBytes(32)]
    }
}
