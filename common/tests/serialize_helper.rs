extern crate transaction;
use transaction::*;

extern crate hex;

use elrond_wasm::types::{Address, BoxedBytes, H256};
use elrond_wasm::elrond_codec::*;
use std::convert::TryInto;

// Run with: cargo test -- --nocapture serialize_transaction
#[test]
fn serialize_transaction() {
    let alice_addr_hex = "0139472eff6886771a982f3083da5d421f24c29181e63888228dc81ca60d69e1";
    let alice_addr_array: [u8; 32] = hex::decode(alice_addr_hex)
        .expect("error decoding alice address")
        .as_slice()
        .try_into()
        .unwrap();

    let bob_addr_hex = "8049d639e5a6980d1cd2392abcce41029cda74a1563523a202f09641cc2618f8";
    let bob_addr_array: [u8; 32] = hex::decode(bob_addr_hex)
        .expect("error decoding bob address")
        .as_slice()
        .try_into()
        .unwrap();

    let transaction = Transaction {
        hash: H256::zero(),
        id: 0,
        from_contract_address: Address::from(alice_addr_array),
        to_chain_id: 0x0A,
        to_contract_address: Address::from(bob_addr_array),
        method_name: BoxedBytes::empty(),
        method_args: Vec::new(),
    };

    let serialized = transaction.get_partial_serialized();
    let serialized_as_hex = hex::encode(serialized.as_slice());

    println!("Serialized: {}", serialized_as_hex);

    // To Also get the hash, go here and input as hex: https://emn178.github.io/online-tools/sha256.html
}
