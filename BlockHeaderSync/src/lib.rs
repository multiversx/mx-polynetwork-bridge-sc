
#![no_std]

imports!();

use header::*;
use header::peer_config::*;

#[elrond_wasm_derive::contract(BlockHeaderSyncImpl)]
pub trait BlockHeaderSync {
    #[init]
    fn init(&self) {
        
    }

    // endpoints

    #[endpoint(SyncGenesisHeader)]
    fn sync_genesis_header(&self, header: &Header) -> SCResult<()> {
        require!(self.is_empty_genesis_header(), "Genesis header already set!");

        require!(!header.consensus_payload.is_empty() || header.is_start_of_epoch(),
            "Invalid genesis header!");

        

        self.set_genesis_header(header);

        Ok(())
    }

    // storage

    // genesis header

    #[storage_get("genesisHeader")]
    fn get_genesis_header(&self) -> Header;

    #[storage_set("genesisHeader")]
    fn set_genesis_header(&self, header: &Header);

    #[storage_is_empty("genesisHeader")]
    fn is_empty_genesis_header(&self) -> bool;

    // consensus peers

    #[storage_get("consensusPeers")]
    fn get_consensus_peers(&self, chain_id: u64, height: u32) -> Vec<PeerConfig>;

    #[storage_set("consensusPeers")]
    fn set_consensus_peers(&self, chain_id: u64, height: u32, peers: &[PeerConfig]);
}
