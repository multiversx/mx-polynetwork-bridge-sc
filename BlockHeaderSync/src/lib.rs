#![no_std]

use header::peer_config::*;
use header::*;

use util::*;

use public_key::*;
use signature::*;

elrond_wasm::imports!();

#[elrond_wasm_derive::contract]
pub trait BlockHeaderSync {
    #[init]
    fn init(&self) {}

    // endpoints

    #[endpoint(syncGenesisHeader)]
    fn sync_genesis_header(&self, header: Header) -> SCResult<()> {
        only_owner!(self, "Only owner can sync genesis header");
        require!(
            self.consensus_peers(header.chain_id).is_empty(),
            "Genesis header already set"
        );

        self.try_update_consensus_peer(&header)?;
        self.block_header_sync_event(&header);

        Ok(())
    }

    #[endpoint(syncBlockHeader)]
    fn sync_block_header(
        &self,
        header: Header,
        header_hash: H256,
        book_keepers: Vec<PublicKey>,
        sig_data: Vec<Signature>,
    ) -> SCResult<()> {
        require!(
            !self.consensus_peers(header.chain_id).is_empty(),
            "Must set genesis header first"
        );

        self.verify_header(&header, &header_hash, &book_keepers, &sig_data)?;
        self.try_update_consensus_peer(&header)?;
        self.block_header_sync_event(&header);

        Ok(())
    }

    // private

    fn verify_header(
        &self,
        header: &Header,
        header_hash: &H256,
        book_keepers: &[PublicKey],
        sig_data: &[Signature],
    ) -> SCResult<()> {
        let prev_consensus = self.consensus_peers(header.chain_id).get();

        for bk in book_keepers {
            let mut serialized_key = Vec::new();
            let _ = bk.dep_encode(&mut serialized_key);
            let key_id = hex_converter::byte_slice_to_hex(&serialized_key);

            // if key doesn't exist, something is wrong
            require!(
                prev_consensus.iter().any(|p| p.id == key_id),
                "Invalid pubkey!"
            );
        }

        self.verify_multi_signature(
            &header_hash.as_bytes().into(),
            book_keepers,
            2 * prev_consensus.len() / 3,
            sig_data,
        )
    }

    fn try_update_consensus_peer(&self, header: &Header) -> SCResult<()> {
        if let Some(chain_config) = &header.consensus_payload.new_chain_config {
            require!(
                !chain_config.peers.is_empty(),
                "Consensus peer list is empty!"
            );
            self.consensus_peers(header.chain_id)
                .set(&chain_config.peers);
        }

        Ok(())
    }

    fn verify(&self, public_key: &PublicKey, data: &BoxedBytes, signature: &Signature) -> bool {
        if data.is_empty() {
            return false;
        }

        self.crypto().verify_secp256k1(
            public_key.value_as_slice(),
            data.as_slice(),
            signature.value_as_slice(),
        )
    }

    fn verify_multi_signature(
        &self,
        data: &BoxedBytes,
        keys: &[PublicKey],
        min_sigs: usize,
        sigs: &[Signature],
    ) -> SCResult<()> {
        require!(sigs.len() >= min_sigs, "Not enough signatures!");

        let mut mask = Vec::new();
        mask.resize(keys.len(), false);

        for sig in sigs {
            let mut valid = false;

            for j in 0..keys.len() {
                if mask[j] {
                    continue;
                }
                if self.verify(&keys[j], data, sig) {
                    mask[j] = true;
                    valid = true;

                    break;
                }
            }

            require!(valid, "Multi-signature verification failed!");
        }

        Ok(())
    }

    // events

    #[event("blockHeaderSyncEvent")]
    fn block_header_sync_event(&self, header: &Header);

    // storage

    #[view(getConsensusPeers)]
    #[storage_mapper("consensusPeers")]
    fn consensus_peers(&self, chain_id: u64) -> SingleValueMapper<Self::Storage, Vec<PeerConfig>>;
}
