use base64::Engine;

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
