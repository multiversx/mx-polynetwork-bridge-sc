#![no_std]

use elrond_wasm::{Address, BoxedBytes, H256, derive_imports};
use elrond_wasm::elrond_codec::*;

use zero_copy_sink::*;
use zero_copy_source::*;

derive_imports!();

#[derive(NestedDecode, NestedEncode, TopDecode, TopEncode, TypeAbi, PartialEq)]
pub enum TransactionStatus {
	None,
	Pending,
	InProgress,
	OutOfFunds,
	Executed,
	Rejected,
}

#[derive(TypeAbi)]
pub struct Transaction {
	pub tx_hash: H256,
	pub tx_id: u64,
	pub from_contract_address: Address,
	pub to_chain_id: u64,
	pub to_contract_address: Address,
	pub method_name: BoxedBytes,
	pub method_args: Vec<BoxedBytes>,
}

impl Transaction {
	pub fn get_partial_serialized(&self) -> BoxedBytes {
		self.serialize_partial().get_sink()
	}
}

// private methods
impl Transaction {
	fn serialize_partial(&self) -> ZeroCopySink {
		let mut sink = ZeroCopySink::new();

		sink.write_elrond_address(&self.from_contract_address);
		sink.write_u64(self.to_chain_id);
		sink.write_elrond_address(&self.to_contract_address);
		sink.write_var_bytes(self.method_name.as_slice());

		sink.write_var_uint(self.method_args.len() as u64);
		for arg in &self.method_args {
			sink.write_var_bytes(arg.as_slice());	
		}

		sink
	}
}


impl NestedEncode for Transaction {
	fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
		let mut sink = ZeroCopySink::new();
		
		sink.write_hash(&self.tx_hash);
		sink.write_u64(self.tx_id);
		
		sink.write_bytes(self.serialize_partial().get_sink().as_slice());

		dest.write(sink.get_sink().as_slice());

		Ok(())
	}
}

impl NestedDecode for Transaction {
	fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
		let mut source = ZeroCopySource::new(input.flush());

		let tx_hash;
		let tx_id;
		let from_contract_address;
		let to_chain_id;
		let to_contract_address;
		let method_name;
		let mut method_args = Vec::new();

		match source.next_hash() {
			Some(val) => tx_hash = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u64() {
			Some(val) => tx_id = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_elrond_address() {
			Some(val) => from_contract_address = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_u64() {
			Some(val) => to_chain_id = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_elrond_address() {
			Some(val) => to_contract_address = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_var_bytes() {
			Some(val) => method_name = val,
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		match source.next_var_uint() {
			Some(len) => {
				for _ in 0..len {
					match source.next_var_bytes() {
						Some(arg) => method_args.push(arg),
						None => return Err(DecodeError::INPUT_TOO_SHORT)
					}
				}
			},
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

		return Ok(Transaction {
			tx_hash,
			tx_id,
			from_contract_address,
			to_chain_id,
			to_contract_address,
			method_name,
			method_args,
		});
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
