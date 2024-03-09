use chain_data::ChainFunction;
use ethabi::{Function, Param, ParamType, StateMutability};

/// @dev Return the compute unit from a deal
/// function returnComputeUnitFromDeal(bytes32 unitId) external;
pub struct ReturnComputeUnitFromDeal;

impl ChainFunction for ReturnComputeUnitFromDeal {
    fn function() -> Function {
        #[allow(deprecated)]
        Function {
            name: "returnComputeUnitFromDeal".to_string(),
            inputs: vec![Param {
                name: "unitId".to_string(),
                kind: ParamType::FixedBytes(32),
                internal_type: None,
            }],
            outputs: vec![],
            constant: None,
            state_mutability: StateMutability::NonPayable,
        }
    }

    fn result_signature() -> Vec<ParamType> {
        vec![]
    }
}
