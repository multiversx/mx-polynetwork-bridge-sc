#![no_std]

use header::*;
use public_key::*;
use signature::*;

elrond_wasm::imports!();

#[elrond_wasm_derive::contract]
pub trait BlockHeaderSync {
    #[init]
    fn init(&self) {}

    // endpoints

    #[only_owner]
    #[endpoint(syncGenesisHeader)]
    fn sync_genesis_header(&self, header: Header, book_keepers: Vec<PublicKey>) -> SCResult<()> {
        require!(
            self.consensus_peers(header.chain_id).is_empty(),
            "Genesis header already set"
        );

        self.consensus_peers(header.chain_id).set(&book_keepers);
        self.current_epoch_start_height(header.chain_id)
            .set(&header.height);

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

        self.verify_header(header.chain_id, header_hash, sig_data)?;
        self.consensus_peers(header.chain_id).set(&book_keepers);

        if header.is_start_of_epoch {
            self.current_epoch_start_height(header.chain_id)
                .set(&header.height);
        }

        self.block_header_sync_event(&header);

        Ok(())
    }

    #[endpoint(verifyHeader)]
    fn verify_header(
        &self,
        chain_id: u64,
        header_hash: H256,
        sig_data: Vec<Signature>,
    ) -> SCResult<()> {
        let prev_consensus = self.consensus_peers(chain_id).get();
        let min_sigs = self.get_min_signatures(prev_consensus.len());

        self.verify_multi_signature(
            &header_hash.as_bytes().into(),
            &prev_consensus,
            min_sigs,
            &sig_data,
        )
    }

    // private

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

        let mut keeper_signed = Vec::new();
        keeper_signed.resize(keys.len(), false);

        for sig in sigs {
            let mut signature_is_valid = false;

            for i in 0..keys.len() {
                if keeper_signed[i] {
                    continue;
                }
                if self.verify(&keys[i], data, sig) {
                    keeper_signed[i] = true;
                    signature_is_valid = true;

                    break;
                }
            }

            require!(signature_is_valid, "Multi-signature verification failed!");
        }

        Ok(())
    }

    fn get_min_signatures(&self, consensus_size: usize) -> usize {
        2 * consensus_size / 3
    }

    // events

    #[event("blockHeaderSyncEvent")]
    fn block_header_sync_event(&self, header: &Header);

    // storage

    #[storage_mapper("consensusPeers")]
    fn consensus_peers(&self, chain_id: u64) -> SingleValueMapper<Self::Storage, Vec<PublicKey>>;

    #[view(getCurrentEpochStartHeight)]
    #[storage_mapper("currentEpochStartHeight")]
    fn current_epoch_start_height(&self, chain_id: u64) -> SingleValueMapper<Self::Storage, u32>;
}
