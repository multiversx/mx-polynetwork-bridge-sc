use elrond_wasm::BoxedBytes;
use elrond_wasm::elrond_codec::*;

use zero_copy_sink::*;
use zero_copy_source::*;

pub struct PeerConfig {
	pub index: u32,
	pub id: BoxedBytes // string in Go, but prefer byte array in Rust
}

impl NestedEncode for PeerConfig {
	fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
		let mut sink = ZeroCopySink::new();

        sink.write_u32(self.index);
        sink.write_var_bytes(self.id.as_slice());

		dest.write(sink.get_sink().as_slice());

		Ok(())
	}
}

impl NestedDecode for PeerConfig {
	fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
		let mut source = ZeroCopySource::new(input.flush());
		
        let index;
        let id;

		match source.next_u32() {
			Some(val) => index = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_var_bytes() {
            Some(val) => id = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT)
        }

		// if there are bytes left, something went wrong
		if source.get_bytes_left() > 0 {
			return Err(DecodeError::INPUT_TOO_LONG);
		}
		else {
			return Ok(PeerConfig {
                index,
                id
			});
		}
	}
}

impl TopEncode for PeerConfig {
	#[inline]
	fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
		top_encode_from_nested(self, output)
	}
}

impl TopDecode for PeerConfig {
	fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
		top_decode_from_nested(input)
	}
}
