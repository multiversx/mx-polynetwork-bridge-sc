use elrond_wasm::api::BigUintApi;
use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::H256;

use zero_copy_sink::*;
use zero_copy_source::*;

elrond_wasm::derive_imports!();

#[derive(TypeAbi)]
pub struct ToMerkleValue<BigUint: BigUintApi> {
    pub poly_tx_hash: H256,
    pub from_chain_id: u64,
    pub tx: crate::Transaction<BigUint>,
}

impl<BigUint: BigUintApi> NestedEncode for ToMerkleValue<BigUint> {
    fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
        let mut sink = ZeroCopySink::new();

        sink.write_var_bytes(self.poly_tx_hash.as_bytes());
        sink.write_u64(self.from_chain_id);
        self.tx.dep_encode(&mut sink)?;

        dest.write(sink.get_sink().as_slice());

        Ok(())
    }
}

impl<BigUint: BigUintApi> NestedDecode for ToMerkleValue<BigUint> {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let mut source = ZeroCopySource::new(input.flush());

        let poly_tx_hash;
        let from_chain_id;
        let tx;

        match source.next_var_bytes() {
            Some(val) => poly_tx_hash = H256::from_slice(val.as_slice()),
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_u64() {
            Some(val) => from_chain_id = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match crate::Transaction::decode_from_source(&mut source) {
            Result::Ok(val) => tx = val,
            Result::Err(err) => return Err(err),
        }

        Ok(Self {
            poly_tx_hash,
            from_chain_id,
            tx,
        })
    }
}

impl<BigUint: BigUintApi> TopEncode for ToMerkleValue<BigUint> {
    #[inline]
    fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
        top_encode_from_nested(self, output)
    }
}

impl<BigUint: BigUintApi> TopDecode for ToMerkleValue<BigUint> {
    fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
        top_decode_from_nested(input)
    }
}
