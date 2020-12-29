#![no_std]

use elrond_wasm::{Box, derive_imports};

pub const ETH_ADDRESS_LEN: usize = 20;

pub const POLYCHAIN_PUBKEY_LEN: usize = 67;
pub const POLYCHAIN_SIGNATURE_LEN: usize = 65;
pub const POLYCHAIN_EPOCH_HEIGHT: u32 = 60000;

derive_imports!();

#[derive(TypeAbi)]
pub struct EthAddress(Box<[u8; ETH_ADDRESS_LEN]>);
#[derive(TypeAbi)]
pub struct PublicKey(Box<[u8; POLYCHAIN_PUBKEY_LEN]>);

#[derive(TypeAbi)]
pub struct Signature(Box<[u8; POLYCHAIN_SIGNATURE_LEN]>);

impl EthAddress {
    pub fn as_slice(&self) -> &[u8] {
        &(*self.0)[..]
    }
}

impl PublicKey {
    pub fn as_slice(&self) -> &[u8] {
        &(*self.0)[..]
    }
}

impl Signature {
    pub fn as_slice(&self) -> &[u8] {
        &(*self.0)[..]
    }
}

impl<'a> From<&'a [u8]> for EthAddress {
    #[inline]
    fn from(byte_slice: &'a [u8]) -> Self {
        let mut data = [0u8; ETH_ADDRESS_LEN];

        if byte_slice.len() >= ETH_ADDRESS_LEN {
            for i in 0..ETH_ADDRESS_LEN {
                data[i] = byte_slice[i];
            }
        }

        EthAddress(Box::from(data))
    }
}

impl<'a> From<&'a [u8]> for PublicKey {
    #[inline]
    fn from(byte_slice: &'a [u8]) -> Self {
        let mut data = [0u8; POLYCHAIN_PUBKEY_LEN];

        if byte_slice.len() >= POLYCHAIN_PUBKEY_LEN {
            for i in 0..POLYCHAIN_PUBKEY_LEN {
                data[i] = byte_slice[i];
            }
        }

        PublicKey(Box::from(data))
    }
}

impl<'a> From<&'a [u8]> for Signature {
    #[inline]
    fn from(byte_slice: &'a [u8]) -> Self {
        let mut data = [0u8; POLYCHAIN_SIGNATURE_LEN];

        if byte_slice.len() >= POLYCHAIN_SIGNATURE_LEN {
            for i in 0..POLYCHAIN_SIGNATURE_LEN {
                data[i] = byte_slice[i];
            }
        }

        Signature(Box::from(data))
    }
}

// byte slice to hex converter

pub mod hex_converter {
    use elrond_wasm::{BoxedBytes, Vec};

    pub fn half_byte_to_hex_digit(num: u8) -> u8 {
        if num < 10 {
            b'0' + num
        } else {
            b'a' + num - 0xau8
        }
    }

    pub fn byte_to_hex(byte: u8) -> (u8, u8) {
        let digit1 = half_byte_to_hex_digit(byte >> 4);
        let digit2 = half_byte_to_hex_digit(byte & 0x0f);

        (digit1, digit2)
    }

    pub fn byte_slice_to_hex(bytes: &[u8]) -> BoxedBytes {
        let mut hex = Vec::new();

        for b in bytes {
            let byte_hex = byte_to_hex(*b);

            hex.push(byte_hex.0);
            hex.push(byte_hex.1);
        }

        BoxedBytes::from(hex.as_slice())
    }
}
