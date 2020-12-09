use elrond_wasm::{H256, Vec};
use elrond_wasm::elrond_codec::*;

use util::*;
use zero_copy_sink::*;
use zero_copy_source::*;

pub mod chain_config;
pub mod vbft_block_info;

use vbft_block_info::*;

pub struct Header {
    pub version: u32,
    pub chain_id: u64,
    pub prev_block_hash: H256,
    pub transactions_root: H256,
    pub cross_states_root: H256,
    pub block_root: H256,
    pub timestamp: u32,
    pub height: u32,
    pub consensus_data: u64,
    pub consensus_payload: VbftBlockInfo,
	pub next_book_keeper: EthAddress,
	pub book_keepers: Vec<PublicKey>,
	pub sig_data: Vec<Signature>,
	pub block_hash: H256
}

impl NestedEncode for Header {
	fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
		let mut sink = ZeroCopySink::new();

		sink.write_u32(self.version);
		sink.write_u64(self.chain_id);
		sink.write_hash(&self.prev_block_hash);
		sink.write_hash(&self.transactions_root);
		sink.write_hash(&self.cross_states_root);
		sink.write_hash(&self.block_root);
		sink.write_u32(self.timestamp);
		sink.write_u32(self.height);
		sink.write_u64(self.consensus_data);
		
		match self.consensus_payload.dep_encode(dest) {
			Ok(()) => {},
			Err(err) => return Err(err)
		}

		sink.write_eth_address(&self.next_book_keeper);
		
		sink.write_var_uint(self.book_keepers.len() as u64);
		for pubkey in &self.book_keepers {
			sink.write_public_key(pubkey);
		}

		sink.write_var_uint(self.sig_data.len() as u64);
		for sig in &self.sig_data {
			sink.write_signature(sig);
		}

		sink.write_hash(&self.block_hash);

		dest.write(sink.get_sink().as_slice());

		Ok(())
	}
}

impl NestedDecode for Header {
	fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
		let mut source = ZeroCopySource::new(input.flush());

		let version;
		let chain_id;
		let prev_block_hash;
		let transactions_root;
		let cross_states_root;
		let block_root;
		let timestamp;
		let height;
		let consensus_data;
		let consensus_payload;
		let next_book_keeper;
		let mut book_keepers = Vec::new();
		let mut sig_data = Vec::new();
		let block_hash;

		match source.next_u32() {
			Some(val) => version = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u64() {
			Some(val) => chain_id = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_hash() {
			Some(val) => prev_block_hash = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_hash() {
			Some(val) => transactions_root = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_hash() {
			Some(val) => cross_states_root = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_hash() {
			Some(val) => block_root = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u32() {
			Some(val) => timestamp = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u32() {
			Some(val) => height = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u64() {
			Some(val) => consensus_data = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match VbftBlockInfo::dep_decode(input) {
			Ok(val) => consensus_payload = val,
			Err(err) => return Err(err)
		};

		match source.next_eth_address() {
			Some(val) => next_book_keeper = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_var_uint() {
			Some(len) => {
				for _ in 0..len {
					match source.next_public_key() {
						Some(val) => book_keepers.push(val),
						None => return Err(DecodeError::INPUT_TOO_SHORT)
					}
				}
			},
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		}

		match source.next_var_uint() {
			Some(len) => {
				for _ in 0..len {
					match source.next_signature() {
						Some(val) => sig_data.push(val),
						None => return Err(DecodeError::INPUT_TOO_SHORT)
					}
				}
			},
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		}

		match source.next_hash() {
			Some(val) => block_hash = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		// if there are bytes left, something went wrong
		if source.get_bytes_left() > 0 {
			return Err(DecodeError::INPUT_TOO_LONG);
		}
		else {
			return Ok(Header {
				version,
				chain_id,
				prev_block_hash,
				transactions_root,
				cross_states_root,
				block_root,
				timestamp,
				height,
				consensus_data,
				consensus_payload,
				next_book_keeper,
				book_keepers,
				sig_data,
				block_hash,
			});
		}
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
