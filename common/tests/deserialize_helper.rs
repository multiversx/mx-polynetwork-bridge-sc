extern crate transaction;
use transaction::*;

extern crate esdt_payment;
use esdt_payment::*;

extern crate hex;

use elrond_wasm::{elrond_codec::*, BigUintApi};
use elrond_wasm_debug::RustBigUint;

// Run with: cargo test -- --nocapture deserialize_transaction
#[test]
fn deserialize_transaction() {
    let input = "d95c06a936c765969c42846432d41268fd73c7a169e10ad1543050a4431edb0400000000000000000139472eff6886771a982f3083da5d421f24c29181e63888228dc81ca60d69e10a000000000000000139472eff6886771a982f3083da5d421f24c29181e63888228dc81ca60d69e10000";
    let serialized = hex::decode(input).expect("hex decoding failed");
    let transaction = match Transaction::dep_decode(&mut serialized.as_slice()) {
        Ok(tx) => tx,
        Err(_) => panic!("transaction decoding error"),
    };

    println!("Transaction:");
    println!("hash: {}", hex::encode(transaction.hash));
    println!("id: {}", transaction.id);
    println!(
        "from_contract_address: {}",
        hex::encode(transaction.from_contract_address)
    );
    println!("to_chain_id: {}", transaction.to_chain_id);
    println!(
        "to_contract_address: {}",
        hex::encode(transaction.to_contract_address)
    );
    println!(
        "method_name: {}",
        hex::encode(transaction.method_name.as_slice())
    );
    println!("method_args:");
    for i in 0..transaction.method_args.len() {
        println!(
            "Arg{}: {}",
            i,
            hex::encode(transaction.method_args[i].as_slice())
        );
    }
}

// Run with: cargo test -- --nocapture deserialize_esdt_payment
#[test]
fn deserialize_esdt_payment() {
    let input = "0139472eff6886771a982f3083da5d421f24c29181e63888228dc81ca60d69e10139472eff6886771a982f3083da5d421f24c29181e63888228dc81ca60d69e10c5745474c442d653737386363084563918244f40000";
    let serialized = hex::decode(input).expect("hex decoding failed");
    let esdt_payment = match EsdtPayment::<RustBigUint>::dep_decode(&mut serialized.as_slice()) {
        Ok(payment) => payment,
        Err(_) => panic!("esdt payment decoding error"),
    };

    println!("Esdt Payment:");
    println!("sender: {}", hex::encode(esdt_payment.sender));
    println!("receiver: {}", hex::encode(esdt_payment.receiver));
    println!(
        "token_identifier: {}",
        hex::encode(esdt_payment.token_identifier.as_slice())
    );
    println!(
        "amount: {}",
        hex::encode(esdt_payment.amount.to_bytes_be().as_slice())
    );
}
