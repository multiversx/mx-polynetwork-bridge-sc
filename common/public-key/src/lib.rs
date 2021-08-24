#![no_std]

use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::Box;

elrond_wasm::derive_imports!();

const PUBKEY_LENGTH: usize = 67;
const PUBKEY_COMPRESSED_LENGTH: usize = 35;
const PUBKEY_HEADER_LENGTH: usize = 2;

#[derive(TypeAbi, PartialEq, Debug)]
pub struct PublicKey(Box<[u8; PUBKEY_LENGTH]>);

impl PublicKey {
    pub fn value_as_slice(&self) -> &[u8] {
        &(*self.0)[..]
    }

    /// PublicKey has 2 bytes as "header". This function returns the raw key used for signature verification.
    pub fn as_key(&self) -> &[u8] {
        &(&self.0)[PUBKEY_HEADER_LENGTH..]
    }

    pub fn compress_key(&self) -> Vec<u8> {
        let mut compressed_key = Vec::new();
        compressed_key.resize(PUBKEY_COMPRESSED_LENGTH, 0);

        compressed_key.copy_from_slice(&(*self.0.clone())[..PUBKEY_COMPRESSED_LENGTH]);

        // parity flag of the Y coordinate
        compressed_key[2] = if self.0[PUBKEY_LENGTH - 1] % 2 == 0 {
            0x02
        } else {
            0x03
        };

        compressed_key
    }
}

impl NestedEncode for PublicKey {
    fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
        dest.write(self.value_as_slice());

        Ok(())
    }
}

impl NestedDecode for PublicKey {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let boxed_array = Box::<[u8; PUBKEY_LENGTH]>::dep_decode(input)?;

        Ok(PublicKey(boxed_array))
    }
}

impl TopEncode for PublicKey {
    #[inline]
    fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
        top_encode_from_nested(self, output)
    }
}

impl TopDecode for PublicKey {
    fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
        top_decode_from_nested(input)
    }
}
