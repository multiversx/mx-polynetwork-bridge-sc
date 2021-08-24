use elrond_wasm::api::BigUintApi;
use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::BoxedBytes;

use zero_copy_sink::*;
use zero_copy_source::*;

elrond_wasm::derive_imports!();

#[derive(TypeAbi, Clone)]
pub struct TransactionArgs<BigUint: BigUintApi> {
    pub asset_hash: BoxedBytes,
    pub dest_address: BoxedBytes,
    pub amount: BigUint,
}

impl<BigUint: BigUintApi> TransactionArgs<BigUint> {
    pub fn decode_from_source(source: &mut ZeroCopySource) -> Result<Self, DecodeError> {
        let asset_hash;
        let dest_address;
        let amount;

        match source.next_var_bytes() {
            Some(val) => asset_hash = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_var_bytes() {
            Some(val) => dest_address = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_u256::<BigUint>() {
            Some(val) => amount = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        Ok(Self {
            asset_hash,
            dest_address,
            amount,
        })
    }
}

impl<BigUint: BigUintApi> NestedEncode for TransactionArgs<BigUint> {
    fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
        let mut sink = ZeroCopySink::new();

        sink.write_var_bytes(self.asset_hash.as_slice());
        sink.write_var_bytes(self.dest_address.as_slice());
        sink.write_u256(&self.amount)?;

        dest.write(sink.get_sink().as_slice());

        Ok(())
    }
}

impl<BigUint: BigUintApi> NestedDecode for TransactionArgs<BigUint> {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let mut source = ZeroCopySource::new(input.flush());

        Self::decode_from_source(&mut source)
    }
}

impl<BigUint: BigUintApi> TopEncode for TransactionArgs<BigUint> {
    #[inline]
    fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
        top_encode_from_nested(self, output)
    }
}

impl<BigUint: BigUintApi> TopDecode for TransactionArgs<BigUint> {
    fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
        top_decode_from_nested(input)
    }
}
