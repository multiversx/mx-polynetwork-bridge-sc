#![no_std]
#![allow(non_snake_case)]

use elrond_wasm::elrond_codec::TopEncode;
use header::Header;
use merkle_proof::MerkleProof;
use signature::Signature;
use transaction::*;

elrond_wasm::imports!();

#[elrond_wasm_derive::contract]
pub trait CrossChainManagement {
    // TODO: make upgrade-friendly
    #[init]
    fn init(
        &self,
        header_sync_contract_address: Address,
        own_chain_id: u64,
        transaction_relayer_code: BoxedBytes,
    ) -> SCResult<()> {
        require!(
            self.blockchain()
                .is_smart_contract(&header_sync_contract_address),
            "Provided HeaderSync address is not a smart contract address"
        );

        let deploy_gas = self.blockchain().get_gas_left() / 2;
        let opt_address = self
            .transaction_relayer_proxy(Address::zero())
            .init()
            .with_gas_limit(deploy_gas)
            .deploy_contract(&transaction_relayer_code, CodeMetadata::DEFAULT);

        let transaction_relayer_address = opt_address.ok_or("Transaction Relayer deploy failed")?;
        self.transaction_relayer_contract_address()
            .set(&transaction_relayer_address);

        self.header_sync_contract_address()
            .set(&header_sync_contract_address);
        self.own_chain_id().set(&own_chain_id);

        Ok(())
    }

    // endpoints - owner-only

    #[only_owner]
    #[endpoint(addAddressToApprovedlist)]
    fn add_address_to_approved_list(&self, approved_address: Address) -> SCResult<()> {
        self.approved_address_list().insert(approved_address);

        Ok(())
    }

    #[only_owner]
    #[endpoint(removeAddressFromApprovedlist)]
    fn remove_address_from_approved_list(&self, approved_address: Address) -> SCResult<()> {
        self.approved_address_list().remove(&approved_address);

        Ok(())
    }

    // endpoints - approved addresses only

    /*
    #[endpoint(setOffchainTxStatus)]
    fn set_offchain_tx_status(
        &self,
        poly_tx_hash: H256,
        tx_status: TransactionStatus,
    ) -> SCResult<()> {
        self.require_caller_approved()?;
        require!(
            !self.tx_by_hash(&poly_tx_hash).is_empty(),
            "Transaction does not exist"
        );
        require!(
            self.tx_status(&poly_tx_hash).get() == TransactionStatus::InProgress,
            "Transaction must be in InProgress status"
        );

        self.tx_status(&poly_tx_hash).set(&tx_status);

        match tx_status {
            TransactionStatus::Executed => {
                self.try_burn_payment_for_tx(&poly_tx_hash)?;
            }
            TransactionStatus::Rejected => {
                self.refund_payment_for_tx(&poly_tx_hash);
            }
            _ => return sc_error!("Transaction status may only be set to Executed or Rejected"),
        }

        Ok(())
    }

    /// Gets pending transactions from Elrond -> other_chain
    #[endpoint(getNextPendingCrossChainTx)]
    fn get_next_pending_cross_chain_tx(&self) -> SCResult<Transaction> {
        self.require_caller_approved()?;

        match self.pending_cross_chain_tx_list().pop_front() {
            Some(poly_tx_hash) => {
                self.tx_status(&poly_tx_hash)
                    .set(&TransactionStatus::InProgress);

                Ok(self.tx_by_hash(&poly_tx_hash).get())
            }
            None => sc_error!("No pending transactions exist"),
        }
    }
    */

    #[endpoint(getMerkleProof)]
    fn get_merkle_proof(&self, proof: BoxedBytes, root: H256) -> SCResult<BoxedBytes> {
        let merkle_proof = MerkleProof::from_bytes(self.crypto(), &proof)?;
        let proof_root = merkle_proof.get_proof_root();

        require!(proof_root == root, "Proof root mismatch");

        Ok(merkle_proof.into_raw_leaf())
    }

    // Transaction from other chain -> Elrond
    #[endpoint(verifyHeaderAndExecuteTx)]
    fn verify_header_and_execute_tx(
        &self,
        tx_proof: BoxedBytes,
        raw_tx_header: BoxedBytes,
        current_header_proof: BoxedBytes,
        raw_current_header: BoxedBytes,
        header_sigs: Vec<Signature>,
    ) -> SCResult<()> {
        self.require_caller_approved()?;

        let tx_header_hash = Header::hash_raw_header(self.crypto(), &raw_tx_header);
        let tx_header = Header::top_decode(raw_tx_header.as_slice())?;

        let current_header_hash = Header::hash_raw_header(self.crypto(), &raw_current_header);
        let current_header = Header::top_decode(raw_current_header.as_slice())?;

        let block_header_sync_address = self.header_sync_contract_address().get();

        let epoch_start_height = self
            .block_header_sync_proxy(block_header_sync_address.clone())
            .current_epoch_start_height()
            .execute_on_dest_context();

        // since the verify method returns SCResult<()>, the whole call will crash if the verify fails
        if tx_header.height >= epoch_start_height {
            self.block_header_sync_proxy(block_header_sync_address)
                .verify_header(tx_header_hash, header_sigs)
                .execute_on_dest_context();
        } else {
            self.block_header_sync_proxy(block_header_sync_address)
                .verify_header(current_header_hash.clone(), header_sigs)
                .execute_on_dest_context();

            let current_header_merkle_proof =
                MerkleProof::from_bytes(self.crypto(), &current_header_proof)?;

            require!(
                current_header_merkle_proof.get_proof_root() == current_header.block_root,
                "Current header merkle proof failed: Block Root does not match"
            );

            let proven_hash = current_header_merkle_proof.into_raw_leaf();
            require!(
                proven_hash.as_slice() == current_header_hash.as_bytes(),
                "Current header merkle proof failed: hash does not match proven value"
            );
        }

        let tx_merkle_proof = MerkleProof::from_bytes(self.crypto(), &tx_proof)?;

        require!(
            tx_merkle_proof.get_proof_root() == tx_header.cross_state_root,
            "Tx merkle proof failed: Cross State Root does not match"
        );

        let tx_raw = tx_merkle_proof.into_raw_leaf();
        let to_merkle_value = ToMerkleValue::top_decode(tx_raw.as_slice())?;

        require!(
            to_merkle_value.tx.method_name == transaction_relayer::UNLOCK_METHOD_NAME.into(),
            "Only unlock method may be called"
        );

        require!(
            !self.does_tx_exist(to_merkle_value.from_chain_id, &to_merkle_value.poly_tx_hash),
            "Transaction was already processed"
        );
        self.set_tx_exists(to_merkle_value.from_chain_id, &to_merkle_value.poly_tx_hash);

        self.receive_tx_event(&to_merkle_value.tx);

        let transaction_relayer_address = self.transaction_relayer_contract_address().get();
        self.transaction_relayer_proxy(transaction_relayer_address)
            .unlock(
                to_merkle_value.tx.method_args,
                to_merkle_value.tx.from_contract_address,
                to_merkle_value.from_chain_id,
            )
            .execute_on_dest_context();

        Ok(())
    }

    // endpoints

    /// Transactions from Elrond -> other_chain
    #[payable("*")]
    #[endpoint(createCrossChainTx)]
    fn create_cross_chain_tx(
        &self,
        to_chain_id: u64,
        to_contract_address: BoxedBytes,
        method_name: BoxedBytes,
        method_args: TransactionArgs<Self::BigUint>,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let transaction_relayer_address = self.transaction_relayer_contract_address().get();
        require!(
            caller == transaction_relayer_address,
            "Only TransactionRelayer SC may call this function"
        );

        let own_chain_id = self.own_chain_id().get();
        require!(
            to_chain_id != own_chain_id,
            "Must send to a chain other than Elrond"
        );

        let mut tx = Transaction {
            source_chain_tx_hash: H256::zero(),
            cross_chain_tx_id: self.get_and_increment_cross_chain_tx_id(),
            from_contract_address: transaction_relayer_address.into_boxed_bytes(),
            to_chain_id,
            to_contract_address,
            method_name,
            method_args,
        };
        tx.source_chain_tx_hash = tx.calculate_hash(self.crypto());

        require!(
            !self.does_tx_exist(own_chain_id, &tx.source_chain_tx_hash),
            "Transaction was already processed"
        );
        self.set_tx_exists(own_chain_id, &tx.source_chain_tx_hash);

        self.create_tx_event(&tx);

        Ok(())
    }

    // private

    fn require_caller_approved(&self) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        require!(
            self.approved_address_list().contains(&caller),
            "Caller is not an approved address"
        );

        Ok(())
    }

    fn get_and_increment_cross_chain_tx_id(&self) -> BoxedBytes {
        self.cross_chain_tx_id().update(|tx_id| {
            let mut serialized = Vec::new();
            let _ = tx_id.top_encode(&mut serialized);

            *tx_id += 1;

            serialized.into()
        })
    }

    fn does_tx_exist(&self, from_chain_id: u64, poly_tx_hash: &H256) -> bool {
        self.tx_exists(from_chain_id, poly_tx_hash).get()
    }

    fn set_tx_exists(&self, from_chain_id: u64, poly_tx_hash: &H256) {
        self.tx_exists(from_chain_id, poly_tx_hash).set(&true);
    }

    // proxies

    #[proxy]
    fn block_header_sync_proxy(
        &self,
        sc_address: Address,
    ) -> block_header_sync::Proxy<Self::SendApi>;

    #[proxy]
    fn transaction_relayer_proxy(
        &self,
        sc_address: Address,
    ) -> transaction_relayer::Proxy<Self::SendApi>;

    // events

    // for tx from Elrond to another chain
    #[event("createTransaction")]
    fn create_tx_event(&self, tx: &Transaction<Self::BigUint>);

    // for tx from another chain to Elrond
    #[event("receiveTransaction")]
    fn receive_tx_event(&self, tx: &Transaction<Self::BigUint>);

    // storage

    #[storage_mapper("headerSyncContractAddress")]
    fn header_sync_contract_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[storage_mapper("transactionRelayerContractAddress")]
    fn transaction_relayer_contract_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getOwnChainId)]
    #[storage_mapper("ownChainId")]
    fn own_chain_id(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[view(getCrossChainTxId)]
    #[storage_mapper("crossChainTxId")]
    fn cross_chain_tx_id(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("txExists")]
    fn tx_exists(
        &self,
        from_chain_id: u64,
        poly_tx_hash: &H256,
    ) -> SingleValueMapper<Self::Storage, bool>;

    // Approved address list - These addresses can mark transactions as executed/rejected
    // which triggers a burn/refund respectively
    // Only for Elrond -> other_chain transactions
    #[storage_mapper("approvedAddressList")]
    fn approved_address_list(&self) -> SafeSetMapper<Self::Storage, Address>;
}
