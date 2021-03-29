extern crate transaction;
use transaction::*;

extern crate esdt_payment;
use esdt_payment::*;

extern crate hex;

use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::{Address, BoxedBytes, H256, TokenIdentifier};
use std::convert::TryInto;

use elrond_wasm_debug::api::RustBigUint;

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

    let mut args = Vec::new();
    args.push(BoxedBytes::from(&b"argument"[..]));
    args.push(BoxedBytes::from(&[0x02, 0x01, 0x01][..]));
    args.push(BoxedBytes::from(&[0x64 as u8][..]));

    let transaction = Transaction {
        hash: H256::from(hash_as_array),
        id: 0,
        from_contract_address: Address::zero(),
        to_chain_id: 0x2A,
        to_contract_address: Address::from(alice_addr_array),
        method_name: BoxedBytes::from(&b"function_name"[..]),
        method_args: args,
    };

    let mut serialized = Vec::new();
    match transaction.dep_encode(&mut serialized) {
        Ok(()) => {}
        Err(_) => panic!("serialize error"),
    };

    let serialized_as_hex = hex::encode(serialized.as_slice());
    println!("Serialized: {}", serialized_as_hex);
}

// Run with: cargo test -- --nocapture serialize_esdt_payment
#[test]
fn serialize_esdt_payment() {
    let alice_addr_hex = "0139472eff6886771a982f3083da5d421f24c29181e63888228dc81ca60d69e1";
    let alice_addr_array: [u8; 32] = hex::decode(alice_addr_hex)
        .expect("error decoding alice address")
        .as_slice()
        .try_into()
        .unwrap();

    let esdt_payment = EsdtPayment {
        sender: Address::from(alice_addr_array),
        receiver: Address::zero(),
        token_id: TokenIdentifier::from(&b"WrappedEgld"[..]),
        amount: RustBigUint::from(10_000_000 as u64)
    };

    let mut serialized = Vec::new();
    match esdt_payment.dep_encode(&mut serialized) {
        Ok(()) => {}
        Err(_) => panic!("serialize error"),
    };

    let serialized_as_hex = hex::encode(serialized.as_slice());
    println!("Serialized: {}", serialized_as_hex);
}
