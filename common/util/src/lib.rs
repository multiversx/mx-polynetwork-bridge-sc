#![no_std]

pub const POLYCHAIN_EPOCH_HEIGHT: u32 = 60_000;

elrond_wasm::derive_imports!();

// byte slice to hex converter

pub mod hex_converter {
    use elrond_wasm::types::{BoxedBytes, Vec};

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
