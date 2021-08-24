#![no_std]

use eth_address::EthAddress;
use header::*;
use public_key::*;
use signature::*;
use zero_copy_sink::ZeroCopySink;

const MIN_CONSENSUS_SIZE: usize = 3;

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
            self.consensus_peers().is_empty(),
            "Genesis header already set"
        );

        self.consensus_peers().set(&book_keepers);
        self.current_epoch_start_height().set(&header.height);

        self.block_header_sync_event(&header);

        Ok(())
    }

    #[endpoint(syncBlockHeader)]
    fn sync_block_header(
        &self,
        raw_header: BoxedBytes,
        book_keepers: Vec<PublicKey>,
        sig_data: Vec<Signature>,
    ) -> SCResult<()> {
        require!(
            !self.consensus_peers().is_empty(),
            "Must set genesis header first"
        );

        let header_hash = Header::hash_raw_header(self.crypto(), &raw_header);
        let header = Header::top_decode(raw_header.as_slice())?;

        let current_epoch_start_height = self.current_epoch_start_height().get();
        require!(
            header.height > current_epoch_start_height,
            "Header height too low"
        );

        require!(
            book_keepers.len() > MIN_CONSENSUS_SIZE,
            "New consensus has too few members"
        );

        self.verify_header(header_hash, sig_data)?;

        let (next_book_keeper, _) = self.get_next_bookkeeper(&book_keepers);
        require!(
            header.next_book_keeper == next_book_keeper,
            "NextBookkeeper mismatch"
        );

        self.consensus_peers().set(&book_keepers);
        self.current_epoch_start_height().set(&header.height);

        self.block_header_sync_event(&header);

        Ok(())
    }

    #[endpoint(verifyHeader)]
    fn verify_header(&self, header_hash: H256, sig_data: Vec<Signature>) -> SCResult<()> {
        let prev_consensus = self.consensus_peers().get();
        let min_sigs = self.get_min_signatures(prev_consensus.len());

        self.verify_multi_signature(
            &header_hash.as_bytes().into(),
            &prev_consensus,
            min_sigs,
            &sig_data,
        )
    }

    #[view(getHashForHeader)]
    fn get_hash_for_header(&self, raw_header: BoxedBytes) -> H256 {
        Header::hash_raw_header(self.crypto(), &raw_header)
    }

    // private

    fn verify(&self, public_key: &PublicKey, data: &BoxedBytes, signature: &Signature) -> bool {
        if data.is_empty() {
            return false;
        }

        self.crypto().verify_secp256k1(
            public_key.as_key(),
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
        2 * consensus_size / 3 + 1
    }

    fn get_next_bookkeeper(&self, public_keys: &[PublicKey]) -> (EthAddress, Vec<EthAddress>) {
        let keys_len = public_keys.len();
        let min_sigs = self.get_min_signatures(keys_len);
        let mut sink = ZeroCopySink::new();
        let mut keepers = Vec::with_capacity(keys_len);

        sink.write_u16(keys_len as u16);

        for pub_key in public_keys {
            let compressed_key = pub_key.compress_key();
            let hash = self.crypto().keccak256(pub_key.as_key());

            sink.write_var_bytes(compressed_key.as_slice());
            keepers.push(EthAddress::from(hash.as_bytes()));
        }

        sink.write_u16(min_sigs as u16);

        let sha256_hash = self.crypto().sha256(sink.get_sink().as_slice());
        let ripemd160_hash = self.crypto().ripemd160(sha256_hash.as_bytes());
        let next_book_keeper = EthAddress::from(*ripemd160_hash);

        (next_book_keeper, keepers)
    }

    // events

    #[event("blockHeaderSyncEvent")]
    fn block_header_sync_event(&self, header: &Header);

    // storage

    #[storage_mapper("consensusPeers")]
    fn consensus_peers(&self) -> SingleValueMapper<Self::Storage, Vec<PublicKey>>;

    #[view(getCurrentEpochStartHeight)]
    #[storage_mapper("currentEpochStartHeight")]
    fn current_epoch_start_height(&self) -> SingleValueMapper<Self::Storage, u32>;
}
