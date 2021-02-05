extern crate transaction;
use transaction::*;

extern crate hex;

use elrond_wasm::{elrond_codec::*, Address, BoxedBytes, H256};
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

    let hash_as_hex = "25a4fa887af0bb300e21a4bf8c6a7101a17c2039af36ae9b33b32ee962e64039";
    let hash_as_array: [u8; 32] = hex::decode(hash_as_hex)
        .expect("error decoding hash")
        .as_slice()
        .try_into()
        .unwrap();

    let transaction = Transaction {
        hash: H256::from(hash_as_array),
        id: 0,
        from_contract_address: Address::zero(),
        to_chain_id: 0x2A,
        to_contract_address: Address::from(alice_addr_array),
        method_name: BoxedBytes::empty(),
        method_args: Vec::new(),
    };

    let mut serialized = Vec::new();
    match transaction.dep_encode(&mut serialized) {
        Ok(()) => {}
        Err(_) => panic!("serialize error"),
    };

    let serialized_as_hex = hex::encode(serialized.as_slice());
    println!("Serialized: {}", serialized_as_hex);
}
