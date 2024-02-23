use chain_data::ChainFunction;

/// @dev Returns the commitment status
/// @param commitmentId Commitment id
/// @return status commitment status
/// function getStatus(bytes32 commitmentId) external view returns (CCStatus);
pub struct GetStatusFunction;

impl ChainFunction for GetStatusFunction {
    fn function() -> ethabi::Function {
        #[allow(deprecated)]
        let function = ethabi::Function {
            name: "getStatus".to_string(),
            inputs: vec![ethabi::Param {
                name: "commitmentId".to_string(),
                kind: ethabi::ParamType::FixedBytes(32),
                internal_type: None,
            }],
            outputs: vec![],
            constant: None,
            state_mutability: ethabi::StateMutability::View,
        };
        function
    }

    fn signature() -> Vec<ethabi::ParamType> {
        vec![ethabi::ParamType::FixedBytes(32)]
    }
}
