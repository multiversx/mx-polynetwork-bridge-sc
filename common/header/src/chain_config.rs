use elrond_wasm::{Vec, derive_imports};
use elrond_wasm::elrond_codec::*;

use zero_copy_sink::*;
use zero_copy_source::*;

use super::peer_config::*;

derive_imports!();

#[derive(TypeAbi)]
pub struct ChainConfig {
	pub version: u32, // software version
	pub view: u32, // config-updated version
	pub network_size: u32,
	pub consensus_quorum: u32,
	pub block_msg_delay: u64, // time.Duration is i64 in Go, but prefer unsigned version in Rust
	pub hash_msg_delay: u64, // time.Duration
	pub peer_handshake_timeout: u64, // time.Duration
	pub peers: Vec<PeerConfig>,
	pub pos_table: Vec<u32>,
	pub max_block_change_view: u32
}

impl NestedEncode for ChainConfig {
	fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
		let mut sink = ZeroCopySink::new();

		sink.write_u32(self.version);
		sink.write_u32(self.view);
		sink.write_u32(self.network_size);
		sink.write_u32(self.consensus_quorum);
		sink.write_u64(self.block_msg_delay);
		sink.write_u64(self.hash_msg_delay);
		sink.write_u64(self.peer_handshake_timeout);
		
		sink.write_var_uint(self.peers.len() as u64);
		for peer in &self.peers {
			match peer.dep_encode(&mut sink) {
				Ok(()) => {},
				Err(err) => return Err(err)
			}
		};

		sink.write_var_uint(self.pos_table.len() as u64);
		for pos in &self.pos_table {
			sink.write_u32(*pos);
		}

		sink.write_u32(self.max_block_change_view);

		dest.write(sink.get_sink().as_slice());

		Ok(())
	}
}

impl NestedDecode for ChainConfig {
	fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
		let mut source = ZeroCopySource::new(input.flush());
		
		let version;
		let view;
		let network_size;
		let consensus_quorum;
		let block_msg_delay;
		let hash_msg_delay;
		let peer_handshake_timeout;
		let mut peers = Vec::new();
		let mut pos_table = Vec::new();
		let max_block_change_view;

		match source.next_u32() {
			Some(val) => version = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u32() {
			Some(val) => view = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u32() {
			Some(val) => network_size = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u32() {
			Some(val) => consensus_quorum = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u64() {
			Some(val) => block_msg_delay = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u64() {
			Some(val) => hash_msg_delay = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u64() {
			Some(val) => peer_handshake_timeout = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_var_uint() {
			Some(len) => {
				for _ in 0..len {
					match PeerConfig::dep_decode(&mut source) {
						Ok(peer) => peers.push(peer),
						Err(err) => return Err(err)
					}
				}
			},
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_var_uint() {
			Some(len) => {
				for _ in 0..len {
					match source.next_u32() {
						Some(val) => pos_table.push(val),
						None => return Err(DecodeError::INPUT_TOO_SHORT)
					}
				}
			}
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		}

		match source.next_u32() {
			Some(val) => max_block_change_view = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		// if there are bytes left, something went wrong
		if source.get_bytes_left() > 0 {
			return Err(DecodeError::INPUT_TOO_LONG);
		}
		else {
			return Ok(ChainConfig {
				version,
				view,
				network_size,
				consensus_quorum,
				block_msg_delay,
				hash_msg_delay,
				peer_handshake_timeout,
				peers,
				pos_table,
				max_block_change_view,
			});
		}
	}
}

impl TopEncode for ChainConfig {
	#[inline]
	fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
		top_encode_from_nested(self, output)
	}
}

impl TopDecode for ChainConfig {
	fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
		top_decode_from_nested(input)
	}
}
