use asynchronous_codec::{Bytes, BytesMut, Decoder};
use libp2p::bytes::Buf;
use std::fmt::{Display, Formatter};
use std::io::Error;
use std::{fmt, io};

const U64_LENGTH: usize = std::mem::size_of::<u64>();

pub struct LengthCodec {
    max_frame_len: usize,
}

impl LengthCodec {
    pub fn new(max_frame_len: usize) -> Self {
        Self { max_frame_len }
    }
}

#[derive(Debug)]
pub struct LengthCodecError {}

impl Display for LengthCodecError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("frame size too big")
    }
}
impl std::error::Error for LengthCodecError {}
impl Decoder for LengthCodec {
    type Item = Bytes;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < U64_LENGTH {
            return Ok(None);
        }

        let mut len_bytes = [0u8; U64_LENGTH];
        len_bytes.copy_from_slice(&src[..U64_LENGTH]);
        let len = u64::from_be_bytes(len_bytes) as usize;

        if len > self.max_frame_len {
            return Err(Error::new(io::ErrorKind::InvalidInput, LengthCodecError {}));
        }

        if src.len() - U64_LENGTH >= len {
            // Skip the length header we already read.
            src.advance(U64_LENGTH);
            Ok(Some(src.split_to(len).freeze()))
        } else {
            Ok(None)
        }
    }
}
