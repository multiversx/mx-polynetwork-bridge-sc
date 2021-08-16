#![no_std]

use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::Box;

elrond_wasm::derive_imports!();

const SIGNATURE_LENGTH: usize = 70;

#[derive(TypeAbi, PartialEq)]
pub struct Signature(Box<[u8; SIGNATURE_LENGTH]>);

impl Signature {
    pub fn value_as_slice(&self) -> &[u8] {
        &(*self.0)[..]
    }
}

impl NestedEncode for Signature {
    fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
        dest.write(self.value_as_slice());

        Ok(())
    }
}

impl NestedDecode for Signature {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let boxed_array = Box::<[u8; SIGNATURE_LENGTH]>::dep_decode(input)?;

        Ok(Signature(boxed_array))
    }
}

impl TopEncode for Signature {
    #[inline]
    fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
        top_encode_from_nested(self, output)
    }
}

impl TopDecode for Signature {
    fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
        top_decode_from_nested(input)
    }
}
