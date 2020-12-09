
#![no_std]

imports!();

use header::*;

#[elrond_wasm_derive::contract(BlockHeaderSyncImpl)]
pub trait BlockHeaderSync {
    #[init]
    fn init(&self) {
        
    }

    // endpoints

    #[endpoint(SyncGenesisHeader)]
    fn sync_genesis_header(&self, _header: &Header) {

    }
}
