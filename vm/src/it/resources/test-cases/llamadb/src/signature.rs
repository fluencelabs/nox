use core::fmt;
use std::error::Error;
use std::num::ParseIntError;

use secp256k1::{verify, Message, PublicKey, Signature};
use sha2::{Digest, Sha256};

#[derive(Debug)]
struct MyError(String);
impl Error for MyError {}
impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

type GenResult<T> = ::std::result::Result<T, Box<Error>>;

lazy_static! {
    static ref PK: PublicKey = get_pk();
}

// Full: 64 + 1 byte prefix
static PUBLIC_KEY: [u8; 65] = [
    0x04, 0xfb, 0x6e, 0x27, 0x79, 0x77, 0xf4, 0x67, 0x61, 0x8a, 0xde, 0x83, 0xf7, 0x50, 0x5b, 0x6f,
    0x44, 0x8b, 0xed, 0x40, 0xff, 0x10, 0x6d, 0xfd, 0xde, 0x56, 0xde, 0x82, 0xfb, 0x14, 0xc7, 0x8a,
    0x53, 0x07, 0x57, 0x8e, 0x60, 0x91, 0x90, 0xd6, 0x5f, 0xc6, 0x39, 0x61, 0x97, 0x0c, 0xf1, 0x48,
    0x62, 0x3f, 0x3d, 0xc8, 0xfc, 0x8e, 0x33, 0x17, 0x7a, 0xa0, 0x5a, 0xdb, 0x4b, 0x78, 0x10, 0x28,
    0x2b,
];

fn get_pk() -> PublicKey {
    PublicKey::parse_slice(&PUBLIC_KEY, None).expect("Invalid public key")
}

fn err_msg(s: &str) -> Box<Error> {
    MyError(s.to_string()).into()
}

fn decode_hex(s: &str) -> GenResult<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<u8>, ParseIntError>>()
        .map_err(Into::into)
}

fn hash_message(message: &str) -> [u8; 32] {
    let mut sha = Sha256::default();
    sha.input(message.as_bytes());
    let hash = sha.result();
    let mut result = [0; 32];
    result.copy_from_slice(hash.as_slice()); //TODO: is there a better way for GenericArray<u8, 32> -> [u8; 32]
    result
}

fn check_signature(hash: &[u8; 32], signature: &str) -> GenResult<bool> {
    let signature = decode_hex(signature)?;
    let signature = Signature::parse_slice(signature.as_slice())
        .map_err(|e| err_msg(&format!("Error parsing signature: {:?}", e)))?;
    let message = Message::parse(hash);

    Ok(verify(&message, &signature, &PK))
}

fn split(input: &String) -> GenResult<(&str, &str)> {
    let pos: usize = input.find("\n").ok_or(err_msg(
        "Invalid input. Should be <signature hex>\\n<sql_query>",
    ))?;
    let signature: &str = &input[..pos];
    let sql_str: &str = &input[pos + 1..];
    Ok((signature, sql_str))
}

pub fn check_input(input: &String) -> GenResult<&str> {
    let (signature, sql_str) = split(input)?;
    let hash = hash_message(sql_str);
    if check_signature(&hash, signature)? {
        Ok(sql_str)
    } else {
        Err(err_msg("Invalid signature"))
    }
}
