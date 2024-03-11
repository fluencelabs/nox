use crate::{next_opt, ChainDataError};
use ethabi::ethereum_types::U256;
use ethabi::Token;

pub trait ChainFunction {
    fn function() -> ethabi::Function;
    fn result_signature() -> Vec<ethabi::ParamType>;

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
        let mut tokens = crate::parse_chain_data(data, &Self::result_signature())?.into_iter();
        next_opt(&mut tokens, "uint", Token::into_uint)
    }

    fn decode_fixed_bytes(data: &str) -> Result<Vec<u8>, ChainDataError> {
        let mut tokens = crate::parse_chain_data(data, &Self::result_signature())?.into_iter();
        next_opt(&mut tokens, "bytes", Token::into_fixed_bytes)
    }

    fn decode_tuple(data: &str) -> Result<Vec<Token>, ChainDataError> {
        crate::parse_chain_data(data, &Self::result_signature())
    }
}
