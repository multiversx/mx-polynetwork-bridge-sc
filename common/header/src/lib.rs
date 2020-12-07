use elrond_wasm::{Box, H256, Vec};
use elrond_wasm::elrond_codec::*;

use util::*;

// TO DO: Change to reflect the types received in documentation
pub struct Header {
    version: u32,
    chain_id: u64,
    prev_block_hash: H256,
    transactions_root: H256,
    cross_states_root: H256,
    block_root: H256,
    timestamp: u32,
    height: u32,
    consensus_data: u64,
    consensus_payload: Vec<u8>,
    next_book_keeper: EthAddress
}

impl Header {
    
}

/* TO DO: change implementation to using ZeroCopySink and ZeroCopySource

impl NestedEncode for Header {
	fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
		self.version.dep_encode(dest)?;
		self.chain_id.dep_encode(dest)?;
		self.prev_block_hash.dep_encode(dest)?;
		self.transactions_root.dep_encode(dest)?;
		self.cross_states_root.dep_encode(dest)?;
		self.block_root.dep_encode(dest)?;
		self.timestamp.dep_encode(dest)?;
		self.height.dep_encode(dest)?;
        self.consensus_data.dep_encode(dest)?;
        self.consensus_payload.dep_encode(dest)?;
        self.next_book_keeper.dep_encode(dest)?;

		Ok(())
	}

	fn dep_encode_or_exit<O: NestedEncodeOutput, ExitCtx: Clone>(
		&self,
		dest: &mut O,
		c: ExitCtx,
		exit: fn(ExitCtx, EncodeError) -> !,
	) {
        self.version.dep_encode_or_exit(dest, c.clone(), exit);
		self.chain_id.dep_encode_or_exit(dest, c.clone(), exit);
		self.prev_block_hash.dep_encode_or_exit(dest, c.clone(), exit);
		self.transactions_root.dep_encode_or_exit(dest, c.clone(), exit);
		self.cross_states_root.dep_encode_or_exit(dest, c.clone(), exit);
		self.block_root.dep_encode_or_exit(dest, c.clone(), exit);
		self.timestamp.dep_encode_or_exit(dest, c.clone(), exit);
		self.height.dep_encode_or_exit(dest, c.clone(), exit);
        self.consensus_data.dep_encode_or_exit(dest, c.clone(), exit);
        self.consensus_payload.dep_encode_or_exit(dest, c.clone(), exit);
        self.next_book_keeper.dep_encode_or_exit(dest, c.clone(), exit);
	}
}

impl TopEncode for Header {
	#[inline]
	fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
		top_encode_from_nested(self, output)
	}

	#[inline]
	fn top_encode_or_exit<O: TopEncodeOutput, ExitCtx: Clone>(
		&self,
		output: O,
		c: ExitCtx,
		exit: fn(ExitCtx, EncodeError) -> !,
	) {
		top_encode_from_nested_or_exit(self, output, c, exit);
	}
}

impl NestedDecode for Header {
	fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
		Ok(Header {
            version: u32::dep_decode(input)?,
            chain_id: u64::dep_decode(input)?,
            prev_block_hash: H256::dep_decode(input)?,
            transactions_root: H256::dep_decode(input)?,
            cross_states_root: H256::dep_decode(input)?,
            block_root: H256::dep_decode(input)?,
            timestamp: u32::dep_decode(input)?,
            height: u32::dep_decode(input)?,
            consensus_data: u64::dep_decode(input)?,
            consensus_payload: Vec::<u8>::dep_decode(input)?,
            next_book_keeper: BoxArray20::dep_decode(input)?,
		})
	}

	fn dep_decode_or_exit<I: NestedDecodeInput, ExitCtx: Clone>(
		input: &mut I,
		c: ExitCtx,
		exit: fn(ExitCtx, DecodeError) -> !,
	) -> Self {
		Header {
            version: u32::dep_decode_or_exit(input, c.clone(), exit),
            chain_id: u64::dep_decode_or_exit(input, c.clone(), exit),
            prev_block_hash: H256::dep_decode_or_exit(input, c.clone(), exit),
            transactions_root: H256::dep_decode_or_exit(input, c.clone(), exit),
            cross_states_root: H256::dep_decode_or_exit(input, c.clone(), exit),
            block_root: H256::dep_decode_or_exit(input, c.clone(), exit),
            timestamp: u32::dep_decode_or_exit(input, c.clone(), exit),
            height: u32::dep_decode_or_exit(input, c.clone(), exit),
            consensus_data: u64::dep_decode_or_exit(input, c.clone(), exit),
            consensus_payload: Vec::<u8>::dep_decode_or_exit(input, c.clone(), exit),
            next_book_keeper: BoxArray20::dep_decode_or_exit(input, c.clone(), exit),
		}
	}
}

impl TopDecode for Header {
	fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
		top_decode_from_nested(input)
	}

	fn top_decode_or_exit<I: TopDecodeInput, ExitCtx: Clone>(
		input: I,
		c: ExitCtx,
		exit: fn(ExitCtx, DecodeError) -> !,
	) -> Self {
		top_decode_from_nested_or_exit(input, c, exit)
	}
}
*/
