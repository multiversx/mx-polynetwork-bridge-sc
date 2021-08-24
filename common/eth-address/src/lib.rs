#![no_std]

use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::Box;

elrond_wasm::derive_imports!();

pub const ETH_ADDRESS_LENGTH: usize = 20;

#[derive(TypeAbi)]
pub struct EthAddress(Box<[u8; ETH_ADDRESS_LENGTH]>);

impl EthAddress {
    pub fn zero() -> Self {
        EthAddress(Box::from([0u8; ETH_ADDRESS_LENGTH]))
    }
}

impl EthAddress {
    pub fn value_as_slice(&self) -> &[u8] {
        &(*self.0)[..]
    }
}

impl<'a> From<&'a [u8]> for EthAddress {
    fn from(slice: &'a [u8]) -> Self {
        let mut addr = Self::zero();

        if slice.len() >= ETH_ADDRESS_LENGTH {
            (*addr.0).copy_from_slice(&slice[..ETH_ADDRESS_LENGTH])
        }
        
        addr
    }
}

impl From<[u8; ETH_ADDRESS_LENGTH]> for EthAddress {
    fn from(array: [u8; ETH_ADDRESS_LENGTH]) -> Self {
        Self::from(&array[..])
    }
}

impl PartialEq for EthAddress {
    fn eq(&self, other: &EthAddress) -> bool {
        self.value_as_slice() == other.value_as_slice()
    }
}

impl NestedEncode for EthAddress {
    fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
        dest.write(self.value_as_slice());

        Ok(())
    }
}

impl NestedDecode for EthAddress {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let boxed_array = Box::<[u8; ETH_ADDRESS_LENGTH]>::dep_decode(input)?;

        Ok(EthAddress(boxed_array))
    }
}

impl TopEncode for EthAddress {
    #[inline]
    fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
        top_encode_from_nested(self, output)
    }
}

impl TopDecode for EthAddress {
    fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
        top_decode_from_nested(input)
    }
}
