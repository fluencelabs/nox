use crate::ProtocolMessage;
use air_interpreter_sede::{
    define_simple_representation, FromSerialized, MsgPackMultiformat, ToWriter,
};
use asynchronous_codec::{BytesMut, Decoder, Encoder};
use std::io;
use unsigned_varint::codec::UviBytes;

const MAX_BUF_SIZE: usize = 100 * 1024 * 1024;

type ProtocolMessageFormat = MsgPackMultiformat;

define_simple_representation!(
    ProtocolMessageRepresentation,
    ProtocolMessage,
    ProtocolMessageFormat,
    Vec<u8>
);

pub struct FluenceCodec {
    length: UviBytes<BytesMut>,
}

impl FluenceCodec {
    pub fn new() -> Self {
        let mut length: UviBytes<BytesMut> = UviBytes::default();
        length.set_max_len(MAX_BUF_SIZE);
        Self { length }
    }
}

impl Decoder for FluenceCodec {
    type Item = ProtocolMessage;
    type Error = FluenceCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let bytes = self.length.decode(src)?;
        if let Some(bytes) = bytes {
            return ProtocolMessageRepresentation
                .deserialize(&bytes)
                .map(Some)
                .map_err(FluenceCodecError::Deserialize);
        }
        Ok(None)
    }
}

impl Encoder for FluenceCodec {
    type Item<'a> = ProtocolMessage;
    type Error = FluenceCodecError;

    fn encode(&mut self, item: Self::Item<'_>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut msg_buf = vec![];
        ProtocolMessageRepresentation
            .to_writer(&item, &mut msg_buf)
            .map_err(FluenceCodecError::Serialize)?;
        self.length.encode(msg_buf[..].into(), dst)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum FluenceCodecError {
    /// IO error
    Io(std::io::Error),
    /// Length error
    Length(std::io::Error),
    /// CBOR error
    Serialize(<ProtocolMessageFormat as air_interpreter_sede::Format<ProtocolMessage>>::SerializationError),
    Deserialize(<ProtocolMessageFormat as air_interpreter_sede::Format<ProtocolMessage>>::DeserializationError),
}

impl From<std::io::Error> for FluenceCodecError {
    fn from(e: std::io::Error) -> FluenceCodecError {
        FluenceCodecError::Io(e)
    }
}

impl std::error::Error for FluenceCodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FluenceCodecError::Io(ref e) => Some(e),
            FluenceCodecError::Length(ref e) => Some(e),
            FluenceCodecError::Serialize(ref e) => Some(e),
            FluenceCodecError::Deserialize(ref e) => Some(e),
        }
    }
}

impl std::fmt::Display for FluenceCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FluenceCodecError::Io(e) => write!(f, "I/O error: {}", e),
            FluenceCodecError::Length(e) => write!(f, "I/O error: {}", e),
            FluenceCodecError::Serialize(e) => write!(f, "Serialization error: {}", e),
            FluenceCodecError::Deserialize(e) => write!(f, "Deserialization error: {}", e),
        }
    }
}

impl From<FluenceCodecError> for std::io::Error {
    fn from(value: FluenceCodecError) -> Self {
        match value {
            FluenceCodecError::Io(e) => e,
            FluenceCodecError::Length(e) => io::Error::new(io::ErrorKind::InvalidInput, e),
            FluenceCodecError::Serialize(e) => io::Error::new(io::ErrorKind::InvalidInput, e),
            FluenceCodecError::Deserialize(e) => io::Error::new(io::ErrorKind::InvalidInput, e),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::libp2p_protocol::codec::FluenceCodec;
    use crate::{Particle, ProtocolMessage};
    use asynchronous_codec::{BytesMut, Decoder, Encoder};
    use base64::{engine::general_purpose::STANDARD as base64, Engine};
    use libp2p::PeerId;
    use std::str::FromStr;

    #[test]
    fn isomorphic_codec_test() {
        let mut codec = FluenceCodec::new();
        let initial_message = ProtocolMessage::Particle(Particle {
            id: "id".to_string(),
            init_peer_id: PeerId::random(),
            timestamp: 1000,
            ttl: 1000,
            script: "script".to_string(),
            signature: vec![0, 0, 128],
            data: vec![0, 0, 255],
        });
        let mut bytes = BytesMut::new();
        codec
            .encode(initial_message.clone(), &mut bytes)
            .expect("Encoding");

        let result_message = codec.decode(&mut bytes).expect("Decoding");

        assert_eq!(result_message, Some(initial_message))
    }

    #[test]
    fn deserialization_test() {
        let raw_str = "zwKBBIimYWN0aW9uqFBhcnRpY2xlpGRhdGGQomlk2SRkMjA1ZDE0OC00Y2YxLTRlNzYtOGY2ZS1mY\
        2U5ODEwZjVlNmOsaW5pdF9wZWVyX2lk2TQxMkQzS29vV0xMRjdnUUtiNzd4WEhWWm4zS1hhMTR4cDNSQmlBa2JuSzJVQ\
        lJwRGFSOEtipnNjcmlwdNk5KGNhbGwgJWluaXRfcGVlcl9pZCUgKCJnZXREYXRhU3J2IiAiLXJlbGF5LSIpIFtdIC1yZ\
        WxheS0pqXNpZ25hdHVyZdwAQG/MtlwBTizM4UtycW3M4DzM9RPMtsyYGsyNbcy5Msy/zO/MvHoyzL9nFTV4zNgfzNUWz\
        PDMwk7M08zwzMDMoszcFMyqeRnMyD/M9cyXEcz9zJzM8syNzIHM2czNzLXMnMznCql0aW1lc3RhbXDPAAABi/IqldOjd\
        HRsAA==";
        let hex_data = base64.decode(raw_str).expect("Base64");
        let mut bytes = BytesMut::from(&hex_data[..]);

        let mut codec = FluenceCodec::new();

        let result = codec.decode(&mut bytes).expect("Decoding");

        let peer_id = PeerId::from_str("12D3KooWLLF7gQKb77xXHVZn3KXa14xp3RBiAkbnK2UBRpDaR8Kb")
            .expect("Peer id");
        let expected = ProtocolMessage::Particle(Particle {
            id: "d205d148-4cf1-4e76-8f6e-fce9810f5e6c".to_string(),
            init_peer_id: peer_id,
            timestamp: 1700574959059,
            ttl: 0,
            script: "(call %init_peer_id% (\"getDataSrv\" \"-relay-\") [] -relay-)".to_string(),
            signature: vec![
                111, 182, 92, 1, 78, 44, 225, 75, 114, 113, 109, 224, 60, 245, 19, 182, 152, 26,
                141, 109, 185, 50, 191, 239, 188, 122, 50, 191, 103, 21, 53, 120, 216, 31, 213, 22,
                240, 194, 78, 211, 240, 192, 162, 220, 20, 170, 121, 25, 200, 63, 245, 151, 17,
                253, 156, 242, 141, 129, 217, 205, 181, 156, 231, 10,
            ],
            data: vec![],
        });

        assert_eq!(result, Some(expected))
    }
}
