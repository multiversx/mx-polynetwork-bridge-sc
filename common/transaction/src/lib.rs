#![no_std]

use elrond_wasm::api::BigUintApi;
use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::{BoxedBytes, H256};

use zero_copy_sink::*;
use zero_copy_source::*;

elrond_wasm::derive_imports!();

pub mod to_merkle_value;
pub mod transaction_args;

pub use to_merkle_value::*;
pub use transaction_args::*;

#[derive(NestedDecode, NestedEncode, TopDecode, TopEncode, TypeAbi, PartialEq)]
pub enum TransactionStatus {
    None,
    Pending,
    InProgress,
    Executed,
    Rejected,
}

// Not using the built-in Address type for addresses, as not all chains have 32-byte addresses
#[derive(TypeAbi)]
pub struct Transaction {
    pub source_chain_tx_hash: H256,
    pub cross_chain_tx_id: BoxedBytes, // not used
    pub from_contract_address: BoxedBytes,
    pub to_chain_id: u64,
    pub to_contract_address: BoxedBytes,
    pub method_name: BoxedBytes,
    pub method_args: BoxedBytes,
}

impl Transaction {
    pub fn get_partial_serialized(&self) -> BoxedBytes {
        self.serialize_partial().get_sink()
    }

    pub fn decode_method_args<BigUint: BigUintApi>(
        &self,
    ) -> Result<TransactionArgs<BigUint>, DecodeError> {
        let mut source = ZeroCopySource::new(self.method_args.as_slice());

        TransactionArgs::decode_from_source(&mut source)
    }

    pub fn decode_from_source(source: &mut ZeroCopySource) -> Result<Self, DecodeError> {
        let source_chain_tx_hash;
        let cross_chain_tx_id;
        let from_contract_address;
        let to_chain_id;
        let to_contract_address;
        let method_name;
        let method_args;

        match source.next_var_bytes() {
            Some(val) => source_chain_tx_hash = H256::from_slice(val.as_slice()),
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_var_bytes() {
            Some(val) => cross_chain_tx_id = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_var_bytes() {
            Some(val) => from_contract_address = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_u64() {
            Some(val) => to_chain_id = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_var_bytes() {
            Some(val) => to_contract_address = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_var_bytes() {
            Some(val) => method_name = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_var_bytes() {
            Some(val) => method_args = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        return Ok(Transaction {
            source_chain_tx_hash,
            cross_chain_tx_id,
            from_contract_address,
            to_chain_id,
            to_contract_address,
            method_name,
            method_args,
        });
    }
}

// private methods
impl Transaction {
    fn serialize_partial(&self) -> ZeroCopySink {
        let mut sink = ZeroCopySink::new();

        sink.write_var_bytes(self.cross_chain_tx_id.as_slice());
        sink.write_var_bytes(self.from_contract_address.as_slice());
        sink.write_u64(self.to_chain_id);
        sink.write_var_bytes(self.to_contract_address.as_slice());
        sink.write_var_bytes(self.method_name.as_slice());
        sink.write_var_bytes(self.method_args.as_slice());

        sink
    }
}

impl NestedEncode for Transaction {
    fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
        let mut sink = ZeroCopySink::new();

        sink.write_hash(&self.source_chain_tx_hash);
        sink.write_bytes(self.serialize_partial().get_sink().as_slice());

        dest.write(sink.get_sink().as_slice());

        Ok(())
    }
}

impl NestedDecode for Transaction {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let mut source = ZeroCopySource::new(input.flush());

        Self::decode_from_source(&mut source)
    }
}

impl TopEncode for Transaction {
    #[inline]
    fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
        top_encode_from_nested(self, output)
    }
}

impl TopDecode for Transaction {
    fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
        top_decode_from_nested(input)
    }
}
