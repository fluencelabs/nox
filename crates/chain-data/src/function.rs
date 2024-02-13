use ethabi::ethereum_types::U256;

pub trait FunctionTrait {
    fn function() -> ethabi::Function;
    fn signature() -> Vec<ethabi::ParamType>;

    fn data(inputs: &[ethabi::Token]) -> eyre::Result<String> {
        let function = Self::function();
        let data = function.encode_input(inputs)?;
        Ok(format!("0x{}", hex::encode(data)))
    }

    fn data_bytes(inputs: &[ethabi::Token]) -> eyre::Result<Vec<u8>> {
        let function = Self::function();
        Ok(function.encode_input(inputs)?)
    }

    fn decode_uint(data: &str) -> eyre::Result<U256> {
        let mut tokens = crate::parse_chain_data(data, &Self::signature())?;
        let token = tokens.pop().ok_or(eyre::eyre!("No token found"))?;
        Ok(token
            .into_uint()
            .ok_or(eyre::eyre!("Token is not a uint"))?)
    }

    fn decode_bytes(data: &str) -> eyre::Result<Vec<u8>> {
        let mut tokens = crate::parse_chain_data(data, &Self::signature())?;
        let token = tokens.pop().ok_or(eyre::eyre!("No token found"))?;
        Ok(token
            .into_fixed_bytes()
            .ok_or(eyre::eyre!("Token is not bytes32"))?)
    }

    fn decode_tuple(data: &str) -> eyre::Result<Vec<ethabi::Token>> {
        Ok(crate::parse_chain_data(data, &Self::signature())?)
    }
}
