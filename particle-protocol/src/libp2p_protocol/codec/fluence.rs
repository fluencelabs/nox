use crate::ProtocolMessage;
use asynchronous_codec::{BytesMut, Decoder, Encoder, CborCodec, CborCodecError};
use std::io;
use unsigned_varint::codec::UviBytes;

const MAX_BUF_SIZE: usize = 100 * 1024 * 1024;

pub struct FluenceCodec {
    length: UviBytes<BytesMut>,
    cbor: CborCodec<ProtocolMessage, ProtocolMessage>,
}

impl FluenceCodec {
    pub fn new() -> Self {
        let mut length: UviBytes<BytesMut> = UviBytes::default();
        length.set_max_len(MAX_BUF_SIZE);
        let cbor = CborCodec::new();
        Self { length, cbor }
    }
}

impl Decoder for FluenceCodec {
    type Item = ProtocolMessage;
    type Error = FluenceCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let bytes = self.length.decode(src)?;
        if let Some(bytes) = bytes {
            return self
                .cbor
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
        let mut cbor_buf = BytesMut::new();
        self.cbor.encode(item, &mut cbor_buf)?;
        self.length.encode(cbor_buf, dst)?;
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
    Cbor(CborCodecError),
}

impl From<std::io::Error> for FluenceCodecError {
    fn from(e: std::io::Error) -> FluenceCodecError {
        FluenceCodecError::Io(e)
    }
}

impl From<CborCodecError> for FluenceCodecError {
    fn from(e: CborCodecError) -> FluenceCodecError {
        FluenceCodecError::Cbor(e)
    }
}

impl std::error::Error for FluenceCodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FluenceCodecError::Io(ref e) => Some(e),
            FluenceCodecError::Length(ref e) => Some(e),
            FluenceCodecError::Cbor(ref e) => Some(e),
        }
    }
}

impl std::fmt::Display for FluenceCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FluenceCodecError::Io(e) => write!(f, "I/O error: {}", e),
            FluenceCodecError::Length(e) => write!(f, "I/O error: {}", e),
            FluenceCodecError::Cbor(e) => write!(f, "CBOR error: {}", e),
        }
    }
}

impl From<FluenceCodecError> for std::io::Error {
    fn from(value: FluenceCodecError) -> Self {
        match value {
            FluenceCodecError::Io(e) => e,
            FluenceCodecError::Length(e) => io::Error::new(io::ErrorKind::InvalidInput, e),
            FluenceCodecError::Cbor(e) => io::Error::new(io::ErrorKind::InvalidInput, e),
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
        let raw_str = "9QN7ImFjdGlvbiI6IlBhcnRpY2xlIiwiaWQiOiJkMjA1ZDE0OC00Y2YxLTRlNzYtOGY2ZS1\
        mY2U5ODEwZjVlNmMiLCJpbml0X3BlZXJfaWQiOiIxMkQzS29vV0xMRjdnUUtiNzd4WEhWWm4zS1hhMTR4cDNSQmlBa2J\
        uSzJVQlJwRGFSOEtiIiwidGltZXN0YW1wIjoxNzAwNTc0OTU5MDU5LCJ0dGwiOjAsInNjcmlwdCI6IihjYWxsICVpbml\
        0X3BlZXJfaWQlIChcImdldERhdGFTcnZcIiBcIi1yZWxheS1cIikgW10gLXJlbGF5LSkiLCJzaWduYXR1cmUiOlsxMTE\
        sMTgyLDkyLDEsNzgsNDQsMjI1LDc1LDExNCwxMTMsMTA5LDIyNCw2MCwyNDUsMTksMTgyLDE1MiwyNiwxNDEsMTA5LDE\
        4NSw1MCwxOTEsMjM5LDE4OCwxMjIsNTAsMTkxLDEwMywyMSw1MywxMjAsMjE2LDMxLDIxMywyMiwyNDAsMTk0LDc4LDI\
        xMSwyNDAsMTkyLDE2MiwyMjAsMjAsMTcwLDEyMSwyNSwyMDAsNjMsMjQ1LDE1MSwxNywyNTMsMTU2LDI0MiwxNDEsMTI\
        5LDIxNywyMDUsMTgxLDE1NiwyMzEsMTBdLCJkYXRhIjoiIn0=";
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
