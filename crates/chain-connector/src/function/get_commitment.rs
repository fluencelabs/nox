use chain_data::ChainFunction;
use chain_types::Commitment;

/// @dev Returns the commitment info
/// @param commitmentId Commitment id
/// @return info commitment info
/// function getCommitment(bytes32 commitmentId) external view returns (CommitmentView memory);
pub struct GetCommitmentFunction;

impl ChainFunction for GetCommitmentFunction {
    fn function() -> ethabi::Function {
        #[allow(deprecated)]
        ethabi::Function {
            name: "getCommitment".to_string(),
            inputs: vec![ethabi::Param {
                name: "commitmentId".to_string(),
                kind: ethabi::ParamType::FixedBytes(32),
                internal_type: None,
            }],
            outputs: vec![],
            constant: None,
            state_mutability: ethabi::StateMutability::View,
        }
    }
    fn signature() -> Vec<ethabi::ParamType> {
        Commitment::signature()
    }
}
