pub fn decode_hex(h: &str) -> Result<Vec<u8>, hex::FromHexError> {
    let h = h.trim_start_matches("0x");
    hex::decode(h)
}
