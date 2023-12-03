//! A type for storing base64url-encoded strings.  Not currently in use.
use crate::error::Error;
use base64::{display::Base64Display, engine::general_purpose, Engine as _};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Default)]
pub struct Base64(Vec<u8>);

impl Base64 {
    pub fn b64_decode(b64str: &str) -> Result<Vec<u8>, Error> {
        let engine = general_purpose::URL_SAFE_NO_PAD;
        let bytes = engine.decode(b64str)?;
        Ok(bytes)
    }

    pub fn b64_encode(bytes: &[u8]) -> Result<String, Error> {
        let engine = general_purpose::URL_SAFE_NO_PAD;
        let b64str = engine.encode(bytes);
        Ok(b64str)
    }

    pub fn new(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }

    pub fn raw(&self) -> &[u8] {
        &self.0
    }

    pub fn from_vec(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Base64(bytes))
    }

    pub fn from_b64_str(str: &str) -> Result<Self, Error> {
        Ok(Base64(Self::b64_decode(str)?.to_vec()))
    }

    pub fn from_utf8_str(str: &str) -> Result<Self, Error> {
        Ok(Self(str.as_bytes().to_vec()))
    }

    pub fn to_utf8_string(&self) -> Result<String, Error> {
        Ok(String::from_utf8(self.raw().to_vec())?)
    }

    pub fn to_base64url(&self) -> Result<String, Error> {
        let b64 = Self::b64_encode(self.raw())?;
        Ok(b64)
    }
}

impl std::fmt::Display for Base64 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let raw = self.raw();
        let b64url = Base64Display::new(raw, &general_purpose::URL_SAFE_NO_PAD);
        write!(f, "{}", b64url)
    }
}

impl Serialize for Base64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format!("{}", &self))
    }
}

impl<'de> Deserialize<'de> for Base64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Vis;
        impl serde::de::Visitor<'_> for Vis {
            type Value = Base64;

            // Required method for Visitor trait
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a base64 string")
            }

            // Trait member method
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Base64::from_b64_str(v).map_err(|_| de::Error::custom("failed to decode base64 string"))
            }
        }
        deserializer.deserialize_str(Vis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_bytes_size() -> anyhow::Result<()> {
        let b64url_str = "qU0U1v2UyqNzFkVzGUCcKpclyvy5sH9SrCXs86vvDQawBthPAfB_Z3E0gEDK0eHD";

        let decoded_bytes = Base64::b64_decode(b64url_str)?;

        let encoded_b64str = Base64::b64_encode(&decoded_bytes)?;

        // Calculate the memory usage
        let size_of_b64url_str = mem::size_of_val(b64url_str);
        let size_of_raw_bytes = mem::size_of_val(&decoded_bytes);

        println!(
            "Memory usage of base64url string: {} bytes",
            size_of_b64url_str
        );
        println!(
            "Memory usage of decoded bytes: {} bytes",
            size_of_raw_bytes
        );

        assert_eq!(encoded_b64str, b64url_str);
        Ok(())
    }
}
