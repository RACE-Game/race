use base64::Engine;
use std::time::UNIX_EPOCH;

pub fn base64_encode(data: &[u8]) -> String {
    let engine = base64::engine::general_purpose::STANDARD;
    engine.encode(data)
}

pub fn base64_decode(data: &str) -> Result<Vec<u8>, race_core::error::Error> {
    let engine = base64::engine::general_purpose::STANDARD;
    engine
        .decode(data)
        .map_err(|_| race_core::error::Error::DeserializeError)
}

pub fn addr_shorthand(addr: &str) -> String {
    if addr.len() > 6 {
        let l = addr.len();
        addr[(l - 6)..l].to_string()
    } else {
        addr.to_string()
    }
}

pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
