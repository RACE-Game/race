use base64::Engine;

pub fn base64_encode(data: &[u8]) -> String {
    let engine = base64::engine::general_purpose::STANDARD;
    engine.encode(data)
}

pub fn base64_decode(data: &str) -> Result<Vec<u8>, race_api::error::Error> {
    let engine = base64::engine::general_purpose::STANDARD;
    engine
        .decode(data)
        .map_err(|_| race_api::error::Error::DeserializeError)
}

pub fn addr_shorthand(addr: &str) -> String {
    format!("[{}]", &addr[0..3])
}
