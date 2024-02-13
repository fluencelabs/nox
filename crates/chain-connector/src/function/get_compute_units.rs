use chain_data::FunctionTrait;
use chain_types::ComputeUnit;
use ethabi::ParamType;

/// @dev Returns the compute units info of a peer
/// function getComputeUnits(bytes32 peerId) external view returns (ComputeUnitView[] memory);
pub struct GetComputeUnitsFunction;

impl FunctionTrait for GetComputeUnitsFunction {
    fn function() -> ethabi::Function {
        #[allow(deprecated)]
        ethabi::Function {
            name: "getComputeUnits".to_string(),
            inputs: vec![ethabi::Param {
                name: "peerId".to_string(),
                kind: ParamType::FixedBytes(32),
                internal_type: None,
            }],
            outputs: vec![],
            constant: None,
            state_mutability: ethabi::StateMutability::View,
        }
    }
    fn signature() -> Vec<ParamType> {
        vec![ParamType::Array(Box::new(ParamType::Tuple(
            ComputeUnit::signature(),
        )))]
    }
}

#[cfg(test)]
mod tests {
    use crate::GetComputeUnitsFunction;
    use chain_data::FunctionTrait;

    #[tokio::test]
    async fn test_data() {
        let peer_id = "0x6497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b";
        let data = GetComputeUnitsFunction::data(&[ethabi::Token::FixedBytes(
            hex::decode(&peer_id[2..]).unwrap(),
        )])
        .unwrap();
        assert_eq!(
            data,
            "0xb6015c6e6497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b"
        );
    }
}
