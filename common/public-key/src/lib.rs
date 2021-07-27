#![no_std]

use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::Box;

elrond_wasm::derive_imports!();

const PUBKEY_LENGTH: usize = 65;

#[derive(TypeAbi, PartialEq)]
pub struct PublicKey(Box<[u8; PUBKEY_LENGTH]>);

impl PublicKey {
    pub fn value_as_slice(&self) -> &[u8] {
        &(*self.0)[..]
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
        let result = input.read_slice(PUBKEY_LENGTH);
        result.map(|sig| {
            let mut sig_array = [0u8; PUBKEY_LENGTH];
            sig_array.copy_from_slice(sig);

            Self(Box::from(sig_array))
        })
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
