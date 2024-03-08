use chain_data::ChainFunction;
use ethabi::ParamType;

/// function getStatus() public view returns (Status)
pub struct GetDealStatusFunction;

impl ChainFunction for GetDealStatusFunction {
    fn function() -> ethabi::Function {
        #[allow(deprecated)]
        let function = ethabi::Function {
            name: "getStatus".to_string(),
            inputs: vec![],
            outputs: vec![],
            constant: None,
            state_mutability: ethabi::StateMutability::View,
        };
        function
    }

    fn result_signature() -> Vec<ParamType> {
        vec![ParamType::FixedBytes(32)]
    }
}
