//! This crate contains the basics of the protocol
//! - Game handler interface
//! - Randomness implementation
//! - Encryption/decryption implementation

pub mod connection;
pub mod context;
pub mod decision;
pub mod effect;
pub mod encryptor;
pub mod engine;
pub mod error;
pub mod event;
pub mod prelude;
pub mod random;
pub mod secret;
pub mod transport;
pub mod types;

#[cfg(test)]
mod tests {

    use borsh::{self, BorshSerialize};
    #[test]
    fn test() {

        #[derive(BorshSerialize)]
        struct S {
            x: u64,
        }
        let s = S {
            x: 1640966400000
        };
        println!("{:?}", s.try_to_vec());
        assert_eq!(1, 2);
    }
}
