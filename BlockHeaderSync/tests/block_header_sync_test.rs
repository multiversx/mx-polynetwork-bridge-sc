/*
use secp256k1::{Message, PublicKey, Secp256k1, Signature};

#[test]
fn test_verify_multisig() {
    let public_keys = vec![
        None,
        Some(PublicKey::from_slice(&string_to_bytes("04a4f44dd65cbcc52b1d1ac51747378a7f84753b5f7bf2760ca21390ced6b172bbf4d03e2cf4e0e79e46f7a757058d240e542853341e88feb1610ff03ba785cfc1")).expect("key err")),
        Some(PublicKey::from_slice(&string_to_bytes("048247efcfeae0fdf760685d1ac1c083be3ff5e9a4a548bc3a2e98f0434f092483760cb1d3138a9beadf9f784d60604f37f1a51464ba228ec44f89879df1c10e07")).expect("key err")),
        Some(PublicKey::from_slice(&string_to_bytes("04d0d0e883c73d8256cf4314822ddd973c0179b73d8ed3df85aad38d36a8b2b0c7696f0c66330d243b1bc7bc8d05e694b4d642ac68f741d2b7f6ea4037ef46b992")).expect("key err")),
        Some(PublicKey::from_slice(&string_to_bytes("047bd771e68adb88398282e21a8b03c12f64c2351ea49a2ba06a0327c83b239ca9420cf3852f7991d2a53afd008d1f6c356294b83aeeb4aad769f8c95ffeb4d5ac")).expect("key err")),
        None,
        Some(PublicKey::from_slice(&string_to_bytes("042092e34e0176dccf8abb496b833d591d25533469b3caf0e279b9742955dd8fc3899a042cd338e82698b5284720f85b309f2b711c05cb37836488371741168da6")).expect("key err")),
    ];

    let signatures = vec![
        Signature::from_der(&string_to_bytes("3044022090eefa6d778ebe346e2360e46d434049875466a167df75bc270f8b70e708cfe102207a0aaf13ba793facea50a35286b939653081e21b16f9ffa25fee454c66cf15cd")).expect("sig err"),
        Signature::from_der(&string_to_bytes("30440220b614723b233746df455331e8fe89c118bd60f2ea8f3e9589611cb683bb92d09b022074c4d3c67b773a6dc01bf704c7565d967a973958f720e6842f3208b229c36fba")).expect("sig err"),
        Signature::from_der(&string_to_bytes("3044022074a7c03a20d272ab4b6984593d6ddefea29844df47547c4f15ab43e7dbc773fb0220290694b75d9dc7fb7ce35071621d70690b6d7563309ea36e7b0ce8f73d11577e")).expect("sig err"),
        Signature::from_der(&string_to_bytes("30440220ddf51b9d12ca13ebd8ac5be98160e4be36954c7f7e9f2f96d3ec5f814bbd4724022038df512077c33def6fd5bf73ef42c669766a370dd5a3d2b44ba23debe9a57a35")).expect("sig err"),
        Signature::from_der(&string_to_bytes("304402200a7b74849d0d51f6acfdfc35bbf1bf014c09fdcfc87d8604fcbe8ed9306ecdf80220189c78e4f68360cb9243fcc212c40890c35e7884034151687bdbbeb316fb9684")).expect("sig err"),
    ];

    let msg = Message::from_slice(&string_to_bytes(
        "433ce4a818afc38000b0c37d24cd9e41a7f96a0687da707860406de797adc2d5",
    ))
    .expect("msg err");

    assert_eq!(verify_multisig(&msg, &public_keys, &signatures), true);
}

fn string_to_bytes(input: &str) -> Vec<u8> {
    hex::decode(input).expect("hex decoding failed")
}

fn verify_multisig(msg: &Message, keys: &[Option<PublicKey>], sigs: &[Signature]) -> bool {
    let secp = Secp256k1::verification_only();
    let mut keeper_signed = Vec::new();
    keeper_signed.resize(keys.len(), false);

    for sig in sigs {
        let mut signature_is_valid = false;

        for i in 0..keys.len() {
            if keeper_signed[i] {
                continue;
            }
            if let Some(key) = keys[i] {
                if secp.verify(msg, &sig, &key).is_ok() {
                    keeper_signed[i] = true;
                    signature_is_valid = true;

                    break;
                }
            }
        }

        if !signature_is_valid {
            return false;
        }
    }

    true
}
*/

use block_header_sync::*;
use elrond_wasm::elrond_codec::{NestedDecode, TopDecode};
use elrond_wasm_debug::TxContext;

use public_key::*;
use signature::*;

#[test]
fn verify_test() {
    let block_header_sync = block_header_sync::contract_obj(TxContext::dummy());

    block_header_sync.init();

    let pubkey1 = deserialize_from_string::<PublicKey>("04ef44beba84422bd76a599531c9fe50969a929a0fee35df66690f370ce19fa8c00ed4b649691d116b7deeb79b714156d18981916e58ae40c0ebacbf3bd0b87877");
    let pubkey2 = deserialize_from_string::<PublicKey>("04a4f44dd65cbcc52b1d1ac51747378a7f84753b5f7bf2760ca21390ced6b172bbf4d03e2cf4e0e79e46f7a757058d240e542853341e88feb1610ff03ba785cfc1");

    let pubkey1_copy = deserialize_from_string::<PublicKey>("04ef44beba84422bd76a599531c9fe50969a929a0fee35df66690f370ce19fa8c00ed4b649691d116b7deeb79b714156d18981916e58ae40c0ebacbf3bd0b87877");
    let pubkey2_copy = deserialize_from_string::<PublicKey>("04a4f44dd65cbcc52b1d1ac51747378a7f84753b5f7bf2760ca21390ced6b172bbf4d03e2cf4e0e79e46f7a757058d240e542853341e88feb1610ff03ba785cfc1");

    let public_keys = vec![
        deserialize_from_string::<PublicKey>("04ef44beba84422bd76a599531c9fe50969a929a0fee35df66690f370ce19fa8c00ed4b649691d116b7deeb79b714156d18981916e58ae40c0ebacbf3bd0b87877"),
        deserialize_from_string::<PublicKey>("04a4f44dd65cbcc52b1d1ac51747378a7f84753b5f7bf2760ca21390ced6b172bbf4d03e2cf4e0e79e46f7a757058d240e542853341e88feb1610ff03ba785cfc1"),
    ];

    let signatures = vec![
        deserialize_from_string::<Signature>("30440220e631bea110252971770367cf76e7b8255ca0bfcaa5bc35468d31c3b72eac364d022076bd89b73879f30c7bd08326558d072e19e6f96cbb808dcbd40e4a209fe7f157"),
        deserialize_from_string::<Signature>("30440220f1376babf31495fbe2433887cdeee92eefd3eb1d31360370ab9d2727161d6bb202207594ffd3568452e0e514d929b6d0f7fedc7e776b6f7cb034e462441a855a5008")
    ];

    let mut keys = Vec::new();
    keys.push(pubkey1);
    keys.push(pubkey2);

    // concatenate keys and signatures, to simulate how actual arguments are passed
    let mut concatenated_keys = Vec::new();
    for key in &keys {
        concatenated_keys.extend_from_slice(key.value_as_slice());
    }

    let mut concatenated_signatures = Vec::new();
    for sig in &signatures {
        concatenated_signatures.extend_from_slice(sig.value_as_slice());
    }

    // try deserialize from concatenated
    match Vec::<PublicKey>::top_decode(concatenated_keys) {
        Result::Ok(keys) => {
            /*assert_eq!(
                keys.len(),
                public_keys.len(),
                "Keys deserialize error, lengths do not match"
            );

            for i in 0..keys.len() {
                assert_eq!(
                    keys[i].value_as_slice(),
                    public_keys[i].value_as_slice(),
                    "Keys mismatch"
                );
            }*/

            assert_eq!(keys[0].value_as_slice(), pubkey1_copy.value_as_slice(), "key 1 failed");
            assert_eq!(keys[1].value_as_slice(), pubkey2_copy.value_as_slice(), "key 2 failed");
        }
        Result::Err(err) => {
            panic!(
                "Vec<PublicKeys> Deserialization error: {}",
                std::str::from_utf8(&err.message_bytes()).unwrap()
            )
        }
    }

    match Vec::<Signature>::top_decode(concatenated_signatures) {
        Result::Ok(sigs) => {
            assert_eq!(
                sigs.len(),
                signatures.len(),
                "Signatures deserialize error, lengths do not match"
            );

            for i in 0..sigs.len() {
                assert_eq!(
                    sigs[i].value_as_slice(),
                    signatures[i].value_as_slice(),
                    "Signatures mismatch"
                );
            }
        }
        Result::Err(err) => {
            panic!(
                "Vec<Signatures> Deserialization error: {}",
                std::str::from_utf8(&err.message_bytes()).unwrap()
            )
        }
    }

    // block_header_sync.verify_multi_signature();
}

// input is in hex format, without "0x" in front
fn deserialize_from_string<T: TopDecode>(input: &str) -> T {
    let serialized = hex::decode(input).expect("hex decoding failed");
    let deserialized = match T::top_decode(serialized.as_slice()) {
        Ok(h) => h,
        Err(err) => panic!(
            "Deserialization error: {}",
            std::str::from_utf8(&err.message_bytes()).unwrap()
        ),
    };

    let test = 5 + 5;

    deserialized
}
