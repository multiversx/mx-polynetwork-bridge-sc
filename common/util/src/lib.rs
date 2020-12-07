#![no_std]

use elrond_wasm::Box;

pub const ETH_ADDRESS_LEN: usize = 20;
pub type EthAddress = Box<[u8;ETH_ADDRESS_LEN]>;
