use chain_data::FunctionTrait;

/// @dev Submits a proof for the commitment
/// @param unitId Compute unit id which provied the proof
/// @param globalUnitNonce The global nonce of the unit for calculating the target hash
/// @param localUnitNonce The local nonce of the unit for calculating the target hash. It's the proof
/// @param targetHash The target hash of this proof
/// function submitProof(bytes32 unitId, bytes32 globalUnitNonce, bytes32 localUnitNonce, bytes32 targetHash) external;

pub struct SubmitProofFunction;

impl FunctionTrait for SubmitProofFunction {
    fn function() -> ethabi::Function {
        #[allow(deprecated)]
        let function = ethabi::Function {
            name: "submitProof".to_string(),
            inputs: vec![
                ethabi::Param {
                    name: "unitId".to_string(),
                    kind: ethabi::ParamType::FixedBytes(32),
                    internal_type: None,
                },
                ethabi::Param {
                    name: "globalUnitNonce".to_string(),
                    kind: ethabi::ParamType::FixedBytes(32),
                    internal_type: None,
                },
                ethabi::Param {
                    name: "localUnitNonce".to_string(),
                    kind: ethabi::ParamType::FixedBytes(32),
                    internal_type: None,
                },
                ethabi::Param {
                    name: "targetHash".to_string(),
                    kind: ethabi::ParamType::FixedBytes(32),
                    internal_type: None,
                },
            ],
            outputs: vec![],
            constant: None,
            state_mutability: ethabi::StateMutability::NonPayable,
        };
        function
    }

    fn signature() -> Vec<ethabi::ParamType> {
        vec![
            ethabi::ParamType::FixedBytes(32),
            ethabi::ParamType::FixedBytes(32),
            ethabi::ParamType::FixedBytes(32),
            ethabi::ParamType::FixedBytes(32),
        ]
    }
}
