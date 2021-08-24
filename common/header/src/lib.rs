#![no_std]

use elrond_wasm::api::CryptoApi;
use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::{BoxedBytes, H256};

use eth_address::*;
use zero_copy_sink::*;
use zero_copy_source::*;

pub mod chain_config;
pub mod peer_config;
pub mod vbft_block_info;

elrond_wasm::derive_imports!();

#[derive(TypeAbi, PartialEq)]
pub struct Header {
    pub version: u32,
    pub chain_id: u64,
    pub prev_block_hash: H256,
    pub transactions_root: H256,
    pub cross_state_root: H256,
    pub block_root: H256,
    pub timestamp: u32,
    pub height: u32,
    pub consensus_data: u64,
    pub consensus_payload: BoxedBytes, // VbftBlockInfo, not used in the SC
    pub next_book_keeper: EthAddress,
}

impl Header {
    pub fn decode_from_source(source: &mut ZeroCopySource) -> Result<Self, DecodeError> {
        let version;
        let chain_id;
        let prev_block_hash;
        let transactions_root;
        let cross_state_root;
        let block_root;
        let timestamp;
        let height;
        let consensus_data;
        let consensus_payload;
        let next_book_keeper;

        match source.next_u32() {
            Some(val) => version = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_u64() {
            Some(val) => chain_id = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_hash() {
            Some(val) => prev_block_hash = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_hash() {
            Some(val) => transactions_root = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_hash() {
            Some(val) => cross_state_root = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_hash() {
            Some(val) => block_root = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_u32() {
            Some(val) => timestamp = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_u32() {
            Some(val) => height = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_u64() {
            Some(val) => consensus_data = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_var_bytes() {
            Some(val) => consensus_payload = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        }

        match source.next_bytes(ETH_ADDRESS_LENGTH) {
            Some(val) => next_book_keeper = EthAddress::from(val.as_slice()),
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        if !source.empty() {
            Err(DecodeError::INPUT_TOO_LONG)
        } else {
            Ok(Header {
                version,
                chain_id,
                prev_block_hash,
                transactions_root,
                cross_state_root,
                block_root,
                timestamp,
                height,
                consensus_data,
                consensus_payload,
                next_book_keeper,
            })
        }
    }

    pub fn hash_raw_header<CA: CryptoApi>(api: CA, raw_header: &BoxedBytes) -> H256 {
        api.sha256(raw_header.as_slice())
    }
}

impl NestedEncode for Header {
    fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
        let mut sink = ZeroCopySink::new();

        sink.write_u32(self.version);
        sink.write_u64(self.chain_id);
        sink.write_hash(&self.prev_block_hash);
        sink.write_hash(&self.transactions_root);
        sink.write_hash(&self.cross_state_root);
        sink.write_hash(&self.block_root);
        sink.write_u32(self.timestamp);
        sink.write_u32(self.height);
        sink.write_u64(self.consensus_data);
        let _ = self.consensus_payload.dep_encode(&mut sink);
        sink.write_bytes(self.next_book_keeper.value_as_slice());

        dest.write(sink.get_sink().as_slice());

        Ok(())
    }
}

impl NestedDecode for Header {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let mut source = ZeroCopySource::new(input.flush());

        Self::decode_from_source(&mut source)
    }
}

impl TopEncode for Header {
    #[inline]
    fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
        top_encode_from_nested(self, output)
    }
}

impl TopDecode for Header {
    fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
        top_decode_from_nested(input)
    }
}
