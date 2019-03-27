use crate::GenResult;
use core::fmt;
use std::num::ParseIntError;

use secp256k1::{verify, Message, PublicKey, Signature};
use sha2::{Digest, Sha256};

#[derive(Debug)]
struct Error(String);

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Representation of a signed request.
/// Format is: signature\nnonce\npayload
/// nonce and payload are concatenated here to avoid reallocation on hashing
struct Signed<'a> {
    signature: &'a str,
    nonce_payload: &'a str,
}

impl<'a> Signed<'a> {
    pub fn payload(&self) -> GenResult<&'a str> {
        let pos = self.nonce_payload.find("\n").ok_or(err_msg(&format!(
            "Invalid input: no \\n between nonce and payload in `{}`. \
             Should be <signature hex>\\n<nonce>\\n<sql_query>",
            self.nonce_payload
        )))?;
        Ok(&self.nonce_payload[pos + 1..])
    }
}

lazy_static! {
    static ref PK: PublicKey = get_pk();
}

/// hard-coded public key, could be replaced directly in a final Wasm binary
// Full: 64 + 1 byte prefix
static PUBLIC_KEY: [u8; 65] = [
    0x04,
    0xba,0x94,0x28,0x52,0xe4,0x35,0x39,0x1b,0x2a,0x6a,0x99,0x3f,0x33,0xf7,0x0c,0x43,
    0x92,0x35,0xf4,0x84,0x15,0xe2,0x5c,0x66,0x8d,0x97,0xd5,0xb0,0xf2,0x63,0xb5,0x4e,
    0x9d,0x98,0xcc,0xae,0xfd,0xc5,0xc6,0xca,0x23,0x7f,0xfc,0x0f,0x2f,0x63,0x35,0x13,
    0x30,0xfa,0xaf,0xe3,0x1d,0x12,0x03,0x79,0xb8,0xdf,0xf9,0x82,0xa3,0x73,0x58,0xe3
];

fn get_pk() -> PublicKey {
    PublicKey::parse_slice(&PUBLIC_KEY, None).expect("Invalid public key")
}

fn err_msg(s: &str) -> Box<Error> {
    Error(s.to_string()).into()
}

/// Converts hex string to a Vec<u8>
fn decode_hex(s: &str) -> GenResult<Vec<u8>> {
    if s.len() % 2 != 0 {
        return Err(err_msg(&format!("Invalid hex length: {} isn't divisible by 2", s.len())))
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<u8>, ParseIntError>>()
        .map_err(Into::into)
}

/// SHA-256 hash of the string
fn hash_message(message: &str) -> [u8; 32] {
    let mut sha = Sha256::default();
    sha.input(message.as_bytes());
    let hash = sha.result();
    let mut result = [0; 32];
    //TODO: is there a better way for GenericArray<u8, 32> -> [u8; 32] ?
    result.copy_from_slice(hash.as_slice());
    result
}

/// Verifies if signature is correct
fn check_signature(hash: &[u8; 32], signature: &str) -> GenResult<bool> {
    let signature = decode_hex(signature)?;
    let signature = Signature::parse_slice(signature.as_slice())
        .map_err(|e| err_msg(&format!("Error parsing signature: {:?}", e)))?;
    let message = Message::parse(hash);

    Ok(verify(&message, &signature, &PK))
}

/// Parse input as `signature\nnonce\npayload`
fn parse_signed(input: &String) -> GenResult<Signed> {
    let pos: usize = input.find("\n").ok_or(err_msg(&format!(
        "Invalid input: no '\\n' between signature and nonce in `{}`. \
         Should be <signature hex>\\n<nonce>\\n<sql_query>",
        input
    )))?;
    let signature: &str = &input[..pos];
    let nonce_payload: &str = &input[pos + 1..];
    Ok(Signed {
        signature,
        nonce_payload,
    })
}

/// Checks if input is signed
/// returns payload string on success
/// throws an error on failure
pub fn check_input(input: &String) -> GenResult<&str> {
    let signed = parse_signed(input)?;
    let hash = hash_message(signed.nonce_payload);
    if check_signature(&hash, signed.signature)? {
        signed.payload()
    } else {
        Err(err_msg("Invalid signature"))
    }
}
