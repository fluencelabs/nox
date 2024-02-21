use crate::{next_opt, ChainDataError};
use ethabi::ethereum_types::U256;
use ethabi::Token;

pub trait FunctionTrait {
    fn function() -> ethabi::Function;
    fn signature() -> Vec<ethabi::ParamType>;

    fn data(inputs: &[Token]) -> Result<String, ChainDataError> {
        let function = Self::function();
        let data = function.encode_input(inputs)?;
        Ok(format!("0x{}", hex::encode(data)))
    }

    fn data_bytes(inputs: &[Token]) -> Result<Vec<u8>, ChainDataError> {
        let function = Self::function();
        Ok(function.encode_input(inputs)?)
    }

    fn decode_uint(data: &str) -> Result<U256, ChainDataError> {
        let mut tokens = crate::parse_chain_data(data, &Self::signature())?.into_iter();
        next_opt(&mut tokens, "uint", Token::into_uint)
    }

    fn decode_bytes(data: &str) -> Result<Vec<u8>, ChainDataError> {
        let mut tokens = crate::parse_chain_data(data, &Self::signature())?.into_iter();
        next_opt(&mut tokens, "bytes", Token::into_bytes)
    }

    fn decode_tuple(data: &str) -> Result<Vec<Token>, ChainDataError> {
        Ok(crate::parse_chain_data(data, &Self::signature())?)
    }
}
