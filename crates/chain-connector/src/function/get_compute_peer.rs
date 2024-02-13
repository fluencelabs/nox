use chain_data::FunctionTrait;
use chain_types::ComputePeer;
use ethabi::{Function, Param, ParamType, StateMutability};

/// struct ComputePeer {
///     bytes32 offerId;
///     bytes32 commitmentId;
///     uint256 unitCount;
///     address owner;
/// }
/// @dev Returns the compute peer info
/// function getComputePeer(bytes32 peerId) external view returns (ComputePeer memory);
pub struct GetComputePeerFunction;

impl FunctionTrait for GetComputePeerFunction {
    fn function() -> Function {
        #[allow(deprecated)]
        Function {
            name: "getComputePeer".to_string(),
            inputs: vec![Param {
                name: String::from("peerId"),
                kind: ParamType::FixedBytes(32),
                internal_type: None,
            }],
            outputs: vec![],
            constant: None,
            state_mutability: StateMutability::View,
        }
    }

    fn signature() -> Vec<ParamType> {
        ComputePeer::signature()
    }
}

#[cfg(test)]
mod tests {
    use crate::GetComputePeerFunction;
    use chain_data::FunctionTrait;

    #[tokio::test]
    async fn test_data() {
        let peer_id = "0x6497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b";
        let data = GetComputePeerFunction::data(&[ethabi::Token::FixedBytes(
            hex::decode(&peer_id[2..]).unwrap(),
        )])
        .unwrap();
        assert_eq!(
            data,
            "0x86e682546497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b"
        );
    }
}
