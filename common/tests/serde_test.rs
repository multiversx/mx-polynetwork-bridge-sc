extern crate header;
use header::*;

use peer_config::*;
use chain_config::*;
use util::*;
use vbft_block_info::*;

use elrond_wasm::{BoxedBytes, H256, Vec, elrond_codec::*};

#[test]
fn peer_config_serde_test() {
    let original = PeerConfig {
        index: 0,
        id: BoxedBytes::from(&b"test_id"[..])
    };

    let mut serialized = Vec::new();
    let _ = original.dep_encode(&mut serialized);
    let expected = [0u8, 0u8, 0u8, 0u8, 7u8, b't', b'e', b's', b't', b'_', b'i', b'd'].to_vec();

    assert_eq!(serialized, expected);

    let deserialized = match PeerConfig::dep_decode(&mut serialized.as_slice()) {
        Ok(des) => des,
        Err(err) => panic!("Deserialization error: {:?}", 
            String::from_utf8(err.message_bytes().to_vec()))
    };

    assert_eq!(deserialized, original);
}

#[test]
fn chain_config_serde_test() {
    let mut peers = Vec::new();
    peers.push(PeerConfig {
        index: 0,
        id: BoxedBytes::from(&b"id0"[..])
    });
    peers.push(PeerConfig {
        index: 1,
        id: BoxedBytes::from(&b"id1"[..])
    });

    let original = ChainConfig {
        version: 5,
        view: 6,
        network_size: 7,
        consensus_quorum: 8,
        block_msg_delay: 1024,
        hash_msg_delay: 2048,
        peer_handshake_timeout: 4096,
        peers: peers,
        pos_table: [0u32, 1u32].to_vec(),
        max_block_change_view: 16
    };
    let mut serialized = Vec::new();
    let _ = original.dep_encode(&mut serialized);
    let expected = [
        5u8, 0u8, 0u8, 0u8, // version
        6u8, 0u8, 0u8, 0u8, // view
        7u8, 0u8, 0u8, 0u8, // network_size
        8u8, 0u8, 0u8, 0u8, // consensus_quorum
        0u8, 4u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // block_msg_delay
        0u8, 8u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // hash_msg_delay
        0u8, 16u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // peer_handshake_timeout
        2u8, // peers len
        0u8, 0u8, 0u8, 0u8, 3u8, b'i', b'd', b'0', // peer[0]
        1u8, 0u8, 0u8, 0u8, 3u8, b'i', b'd', b'1', // peer[1]
        2u8, // pos_table len
        0u8, 0u8, 0u8, 0u8, // pos_table[0]
        1u8, 0u8, 0u8, 0u8, // pos_table[1]
        16u8, 0u8, 0u8, 0u8 // max_block_change_view
    ].to_vec();
    
    assert_eq!(serialized, expected);

    let deserialized = match ChainConfig::dep_decode(&mut serialized.as_slice()) {
        Ok(des) => des,
        Err(err) => panic!("Deserialization error: {:?}", 
            String::from_utf8(err.message_bytes().to_vec()))
    };

    assert_eq!(deserialized, original);
}

#[test]
fn vbft_block_info_test() {
    let mut peers = Vec::new();
    peers.push(PeerConfig {
        index: 0,
        id: BoxedBytes::from(&b"id0"[..])
    });
    peers.push(PeerConfig {
        index: 1,
        id: BoxedBytes::from(&b"id1"[..])
    });

    let original_with_config = VbftBlockInfo {
        proposer: 50,
        vrf_value: BoxedBytes::from(&b"vrf_value"[..]),
        vrf_proof: BoxedBytes::from(&b"vrf_proof"[..]),
        last_config_block_num: 20,
        new_chain_config: Some(ChainConfig {
            version: 5,
            view: 6,
            network_size: 7,
            consensus_quorum: 8,
            block_msg_delay: 1024,
            hash_msg_delay: 2048,
            peer_handshake_timeout: 4096,
            peers: peers,
            pos_table: [0u32, 1u32].to_vec(),
            max_block_change_view: 16
        })
    };

    let mut serialized_with_config = Vec::new();
    let _= original_with_config.dep_encode(&mut serialized_with_config);
    let expected_with_config = [
        50u8, 0u8, 0u8, 0u8, // proposer
        9u8, // vrf_value len
        b'v', b'r', b'f', b'_', b'v', b'a', b'l', b'u', b'e', // vrf_value
        9u8, // vrf_prrof len
        b'v', b'r', b'f', b'_', b'p', b'r', b'o', b'o', b'f', // vrf_proof
        20u8, 0u8, 0u8, 0u8, // last_config_block_num
        1u8, // Some() flag for new_chain_config
        // new chain config
        5u8, 0u8, 0u8, 0u8, // version
        6u8, 0u8, 0u8, 0u8, // view
        7u8, 0u8, 0u8, 0u8, // network_size
        8u8, 0u8, 0u8, 0u8, // consensus_quorum
        0u8, 4u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // block_msg_delay
        0u8, 8u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // hash_msg_delay
        0u8, 16u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // peer_handshake_timeout
        2u8, // peers len
        0u8, 0u8, 0u8, 0u8, 3u8, b'i', b'd', b'0', // peer[0]
        1u8, 0u8, 0u8, 0u8, 3u8, b'i', b'd', b'1', // peer[1]
        2u8, // pos_table len
        0u8, 0u8, 0u8, 0u8, // pos_table[0]
        1u8, 0u8, 0u8, 0u8, // pos_table[1]
        16u8, 0u8, 0u8, 0u8 // max_block_change_view
    ].to_vec();

    assert_eq!(serialized_with_config, expected_with_config);

    let deserialized_with_config = match VbftBlockInfo::dep_decode(
        &mut serialized_with_config.as_slice()) {
        
        Ok(des) => des,
        Err(err) => panic!("Deserialization error: {:?}", 
            String::from_utf8(err.message_bytes().to_vec()))
    };

    assert_eq!(deserialized_with_config, original_with_config);

    // Without config

    let original_without_config = VbftBlockInfo {
        proposer: 50,
        vrf_value: BoxedBytes::from(&b"vrf_value"[..]),
        vrf_proof: BoxedBytes::from(&b"vrf_proof"[..]),
        last_config_block_num: 20,
        new_chain_config: None
    };

    let mut serialized_without_config = Vec::new();
    let _= original_without_config.dep_encode(&mut serialized_without_config);
    let expected_without_config = [
        50u8, 0u8, 0u8, 0u8, // proposer
        9u8, // vrf_value len
        b'v', b'r', b'f', b'_', b'v', b'a', b'l', b'u', b'e', // vrf_value
        9u8, // vrf_prrof len
        b'v', b'r', b'f', b'_', b'p', b'r', b'o', b'o', b'f', // vrf_proof
        20u8, 0u8, 0u8, 0u8, // last_config_block_num
        0u8, // None flag for new_chain_config
    ].to_vec();

    assert_eq!(serialized_without_config, expected_without_config);

    let deserialized_without_config = match VbftBlockInfo::dep_decode(
        &mut serialized_without_config.as_slice()) {
        
        Ok(des) => des,
        Err(err) => panic!("Deserialization error: {:?}", 
            String::from_utf8(err.message_bytes().to_vec()))
    };

    assert_eq!(deserialized_without_config, original_without_config);
}

#[test]
fn header_test() {
    let mut peers = Vec::new();
    peers.push(PeerConfig {
        index: 0,
        id: BoxedBytes::from(&b"id0"[..])
    });
    peers.push(PeerConfig {
        index: 1,
        id: BoxedBytes::from(&b"id1"[..])
    });

    let mut original = Header {
        version: 5,
        chain_id: 6,
        prev_block_hash: H256::from([0x42, 0xa1, 0xff, 0xcd, 0xd6, 0xfd, 0x49, 0xa1, 0xdb, 0xb7, 0xdf, 0x16, 0x36, 0xe6, 0x25, 0xfd, 0xc2, 0xfb, 0x52, 0xc1
            , 0x0c, 0x6e, 0x2b, 0x55, 0xd0, 0x8c, 0xc4, 0x6c, 0xd7, 0x09, 0x70, 0x92]),
        transactions_root: H256::from([0xda, 0x9a, 0x5c, 0x93, 0x0b, 0xd3, 0x5f, 0x48, 0xc7, 0x6d, 0xec, 0xc6, 0xd5, 0x95, 0x30, 0x0a, 0x7e, 0x87, 0xb1, 0x4b
            , 0x5e, 0x91, 0x22, 0x9b, 0xd2, 0x4b, 0x94, 0x48, 0x9a, 0x49, 0x0b, 0x1e]),
        cross_state_root: H256::from([0xf1, 0xce, 0x39, 0x45, 0x33, 0x6f, 0xef, 0x18, 0xe0, 0x4e, 0xe2, 0xd6, 0xae, 0x80, 0xd7, 0xbd, 0xbe, 0x88, 0xb8, 0xe7
            , 0x67, 0x1f, 0xb9, 0x8f, 0xe1, 0xf0, 0x07, 0xbd, 0xf5, 0x06, 0xda, 0xf6]),
        block_root: H256::from([0x55, 0x8c, 0x2f, 0x79, 0x2d, 0x75, 0xfe, 0xe4, 0x36, 0x4c, 0x95, 0x12, 0xcb, 0x22, 0x7b, 0xfa, 0xb6, 0xd9, 0x32, 0x70
            , 0x3a, 0xf3, 0x4b, 0x53, 0xfa, 0x11, 0x70, 0xd9, 0xa6, 0xde, 0x0c, 0x5b]),
        timestamp: 7,
        height: 8,
        consensus_data: 9,
        consensus_payload: Some(VbftBlockInfo {
            proposer: 50,
            vrf_value: BoxedBytes::from(&b"vrf_value"[..]),
            vrf_proof: BoxedBytes::from(&b"vrf_proof"[..]),
            last_config_block_num: 20,
            new_chain_config: Some(ChainConfig {
                version: 5,
                view: 6,
                network_size: 7,
                consensus_quorum: 8,
                block_msg_delay: 1024,
                hash_msg_delay: 2048,
                peer_handshake_timeout: 4096,
                peers: peers,
                pos_table: [0u32, 1u32].to_vec(),
                max_block_change_view: 16
            })}
        ),
        next_book_keeper: EthAddress::from(&[0u8;ETH_ADDRESS_LEN][..]),
        book_keepers: Vec::new(),
        sig_data: Vec::new(),
        block_hash: H256::from([0x26, 0x46, 0x0e, 0xd3, 0x76, 0x17, 0x95, 0x7c, 0x96, 0xd9, 0xab, 0xf5, 0x94, 0xa1, 0xac, 0x86, 0x5a, 0x43, 0x11, 0x02
            , 0xfc, 0x38, 0x77, 0x71, 0xa8, 0xc7, 0x6d, 0xa0, 0x2e, 0x6f, 0x01, 0xe8])
    };

    let mut serialized = Vec::new();
    let _ = original.dep_encode(&mut serialized);
    let mut deserialized = match Header::dep_decode(
        &mut serialized.as_slice()) {
        
        Ok(des) => des,
        Err(err) => panic!("Deserialization error: {:?}", 
            String::from_utf8(err.message_bytes().to_vec()))
    };

    assert_eq!(original, deserialized);

    // without consensus payload
    original.consensus_payload = None;
    serialized.clear();
    let _ = original.dep_encode(&mut serialized);
    deserialized = match Header::dep_decode(
        &mut serialized.as_slice()) {
        
        Ok(des) => des,
        Err(err) => panic!("Deserialization error: {:?}", 
            String::from_utf8(err.message_bytes().to_vec()))
    };

    assert_eq!(original, deserialized);
}
