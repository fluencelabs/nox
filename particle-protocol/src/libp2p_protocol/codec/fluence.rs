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
        if let Some(bytes) = bytes {
            return self
                .json
                .decode(&mut BytesMut::from(&bytes[..]))
                .map_err(Into::into);
        }
        Ok(None)
    }
}

impl Encoder for FluenceCodec {
    type Item<'a> = ProtocolMessage;
    type Error = FluenceCodecError;

    fn encode(&mut self, item: Self::Item<'_>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut json_buf = BytesMut::new();
        self.json.encode(item, &mut json_buf)?;
        self.length.encode(json_buf.freeze(), dst)?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use crate::libp2p_protocol::codec::FluenceCodec;
    use crate::{Particle, ProtocolMessage};
    use asynchronous_codec::{BytesMut, Decoder, Encoder};
    use libp2p::PeerId;

    #[test]
    fn isomorphic_serde_test() {
        let mut codec = FluenceCodec::new();
        let initial_message = ProtocolMessage::Particle(Particle {
            id: "id".to_string(),
            init_peer_id: PeerId::random(),
            timestamp: 1000,
            ttl: 1000,
            script: "script".to_string(),
            signature: vec![0, 0, 128],
            data: vec![0, 0, 128],
        });
        let mut bytes = BytesMut::new();
        codec
            .encode(initial_message.clone(), &mut bytes)
            .expect("Encoding");

        let result_message = codec.decode(&mut bytes).expect("Decoding");

        assert_eq!(result_message, Some(initial_message))
    }
}
