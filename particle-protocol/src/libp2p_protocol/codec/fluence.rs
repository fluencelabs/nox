use crate::libp2p_protocol::codec::length::LengthCodecError;
use crate::libp2p_protocol::codec::LengthCodec;
use crate::ProtocolMessage;
use asynchronous_codec::{BytesMut, Decoder, Encoder, JsonCodec, JsonCodecError};
use std::io;

const MAX_BUF_SIZE: usize = 100 * 1024 * 1024;

pub struct FluenceCodec {
    length: LengthCodec,
    json: JsonCodec<ProtocolMessage, ProtocolMessage>,
}

impl FluenceCodec {
    pub fn new() -> Self {
        let length = LengthCodec::new(MAX_BUF_SIZE);
        let json = JsonCodec::new();
        Self { length, json }
    }
}

impl Decoder for FluenceCodec {
    type Item = ProtocolMessage;
    type Error = FluenceCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let bytes = self.length.decode(src)?;
        if let Some(_) = bytes {
            return self.json.decode(src).map_err(Into::into);
        }
        Ok(None)
    }
}

impl Encoder for FluenceCodec {
    type Item<'a> = ProtocolMessage;
    type Error = FluenceCodecError;

    fn encode(&mut self, _item: Self::Item<'_>, _dst: &mut BytesMut) -> Result<(), Self::Error> {
        todo!()
    }
}

#[derive(Debug)]
pub enum FluenceCodecError {
    /// IO error
    Io(std::io::Error),
    /// Lenght error
    Length(LengthCodecError),
    /// JSON error
    Json(JsonCodecError),
}

impl From<std::io::Error> for FluenceCodecError {
    fn from(e: std::io::Error) -> FluenceCodecError {
        FluenceCodecError::Io(e)
    }
}

impl From<LengthCodecError> for FluenceCodecError {
    fn from(e: LengthCodecError) -> FluenceCodecError {
        FluenceCodecError::Length(e)
    }
}

impl From<JsonCodecError> for FluenceCodecError {
    fn from(e: JsonCodecError) -> FluenceCodecError {
        FluenceCodecError::Json(e)
    }
}

impl std::error::Error for FluenceCodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FluenceCodecError::Io(ref e) => Some(e),
            FluenceCodecError::Length(ref e) => Some(e),
            FluenceCodecError::Json(ref e) => Some(e),
        }
    }
}

impl std::fmt::Display for FluenceCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FluenceCodecError::Io(e) => write!(f, "I/O error: {}", e),
            FluenceCodecError::Length(e) => write!(f, "I/O error: {}", e),
            FluenceCodecError::Json(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl From<FluenceCodecError> for std::io::Error {
    fn from(value: FluenceCodecError) -> Self {
        match value {
            FluenceCodecError::Io(e) => e,
            FluenceCodecError::Length(e) => io::Error::new(io::ErrorKind::InvalidInput, e),
            FluenceCodecError::Json(e) => io::Error::new(io::ErrorKind::InvalidInput, e),
        }
    }
}
