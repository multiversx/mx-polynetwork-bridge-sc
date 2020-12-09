use elrond_wasm::{BoxedBytes};
use elrond_wasm::elrond_codec::*;

use zero_copy_sink::*;
use zero_copy_source::*;

use super::chain_config::*;

pub struct VbftBlockInfo {
	pub proposer: u32,
	pub vrf_value: BoxedBytes,
	pub vrf_proof: BoxedBytes,
	pub last_config_block_num: u32,
	pub new_chain_config: ChainConfig
}

impl NestedEncode for VbftBlockInfo {
	fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
		let mut sink = ZeroCopySink::new();

		sink.write_u32(self.proposer);
		sink.write_var_bytes(self.vrf_value.as_slice());
		sink.write_var_bytes(self.vrf_proof.as_slice());
		sink.write_u32(self.last_config_block_num);
		
		match self.new_chain_config.dep_encode(&mut sink) {
			Ok(()) => {},
			Err(err) => return Err(err)
		};

		dest.write(sink.get_sink().as_slice());

		Ok(())
	}
}

impl NestedDecode for VbftBlockInfo {
	fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
		let mut source = ZeroCopySource::new(input.flush());

		let proposer;
		let vrf_value;
		let vrf_proof;
		let last_config_block_num;
		let new_chain_config;

		match source.next_u32() {
			Some(val) => proposer = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_var_bytes() {
			Some(val) => vrf_value = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_var_bytes() {
			Some(val) => vrf_proof = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u32() {
			Some(val) => last_config_block_num = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match ChainConfig::dep_decode(&mut source) {
			Ok(config) => new_chain_config = config,
			Err(err) => return Err(err)
		};

		// if there are bytes left, something went wrong
		if source.get_bytes_left() > 0 {
			return Err(DecodeError::INPUT_TOO_LONG);
		}
		else {
			return Ok(VbftBlockInfo {
				proposer,
				vrf_value,
				vrf_proof,
				last_config_block_num,
				new_chain_config
			});
		}
	}
}

impl TopEncode for VbftBlockInfo {
	#[inline]
	fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
		top_encode_from_nested(self, output)
	}
}

impl TopDecode for VbftBlockInfo {
	fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
		top_decode_from_nested(input)
	}
}
