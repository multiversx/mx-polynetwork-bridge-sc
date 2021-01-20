#![no_std]

use elrond_wasm::{imports, only_owner, ArgBuffer, HexCallDataSerializer};
use header::*;
use transaction::*;

imports!();

const TRANSFER_ESDT_TO_ACCOUNT_ENDPOINT_NAME: &[u8] = b"transferEsdtToAccount";
const TRANSFER_ESDT_TO_CONTRACT_ENDPOINT_NAME: &[u8] = b"transferEsdtToContract";

#[elrond_wasm_derive::callable(BlockHeaderSyncProxy)]
pub trait BlockHeaderSync {
    #[rustfmt::skip]
	#[callback(get_header_by_height_callback)]
    fn getHeaderByHeight(&self, chain_id: u64, height: u32,
        #[callback_arg] tx: &Transaction,
        #[callback_arg] token_identifier: &BoxedBytes,
        #[callback_arg] amount: &BigUint
    );
}

#[elrond_wasm_derive::contract(CrossChainManagementImpl)]
pub trait CrossChainManagement {
    #[init]
    fn init(&self, header_sync_contract_address: Address, own_chain_id: u64) {
        self.set_header_sync_contract_address(&header_sync_contract_address);
        self.set_own_chain_id(own_chain_id);
    }

    // endpoints - owner-only

    #[endpoint(setTokenManagementContractAddress)]
    fn set_token_management_contract_address_endpoint(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        self.set_token_management_contract_address(&address);

        Ok(())
    }

    #[endpoint(addTokenToWhitelist)]
    fn add_token_to_whitelist(&self, token_identifier: BoxedBytes) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        let mut token_whitelist = self.get_token_whitelist();

        if !token_whitelist.contains(&token_identifier) {
            token_whitelist.push(token_identifier);
        }

        self.set_token_whitelist(&token_whitelist);

        Ok(())
    }

    #[endpoint(removeTokenFromWhitelist)]
    fn remove_token_from_whitelist(&self, token_identifier: BoxedBytes) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        let mut token_whitelist = self.get_token_whitelist();

        for i in 0..token_whitelist.len() {
            if token_whitelist[i] == token_identifier {
                token_whitelist.remove(i);

                break;
            }
        }

        self.set_token_whitelist(&token_whitelist);

        Ok(())
    }

    // endpoints - token manager contract only

    #[endpoint(completeTx)]
    fn complete_tx(&self, poly_tx_hash: H256, tx_status: TransactionStatus) -> SCResult<()> {
        require!(
            !self.is_empty_token_management_contract_address(),
            "token management contract address not set"
        );
        require!(
            self.get_caller() == self.get_token_management_contract_address(),
            "Only the token manager contract may call this"
        );
        require!(
            !self.is_empty_tx_by_hash(&poly_tx_hash),
            "Transaction does not exist"
        );
        require!(
            self.get_tx_status(&poly_tx_hash) == TransactionStatus::InProgress,
            "Transaction must be processed as Pending first"
        );
        require!(
            tx_status == TransactionStatus::OutOfFunds
                || tx_status == TransactionStatus::Executed
                || tx_status == TransactionStatus::Rejected,
            "Transaction status may only be set to OutOfFunds, Executed or Rejected"
        );

        self.set_tx_status(&poly_tx_hash, tx_status);

        Ok(())
    }

    // endpoints

    // TODO: Accept eGLD as payment as well, and automatically wrap it if that's the case
    // TODO: Call EsdtTokenManager and lock the sent tokens there
    #[endpoint(createCrossChainTx)]
    fn create_cross_chain_tx(
        &self,
        to_chain_id: u64,
        to_contract_address: Address,
        method_name: BoxedBytes,
        method_args: Vec<BoxedBytes>,
    ) -> SCResult<()> {
        require!(
            !self.is_empty_token_management_contract_address(),
            "token management contract address not set"
        );

        let token_identifier = self.get_esdt_token_identifier_boxed();
        let esdt_value = self.get_esdt_value_big_uint();
        let tx_id = self.get_cross_chain_tx_id(to_chain_id);

        let from_contract_address = self.get_caller();

        let mut tx = Transaction {
            hash: H256::zero(),
            id: tx_id,
            from_contract_address,
            to_chain_id,
            to_contract_address,
            method_name,
            method_args,
        };
        tx.hash = self.hash_transaction(&tx);

        if !token_identifier.is_empty() && esdt_value > 0 {
            let token_whitelist = self.get_token_whitelist();

            require!(
                token_whitelist.contains(&token_identifier),
                "Token is not on whitelist. Transaction rejected"
            );

            self.set_payment_for_tx(&tx.hash, &(token_identifier, esdt_value));
        }

        self.set_tx_by_hash(&tx.hash, &tx);
        // TODO: Add a way to mark these as Executed/Rejected by an approved address
        self.set_tx_status(&tx.hash, TransactionStatus::Pending);
        self.set_cross_chain_tx_id(to_chain_id, tx_id + 1);

        self.create_tx_event(&tx);

        Ok(())
    }

    #[endpoint(getTxByHash)]
    fn get_tx_by_hash_or_none(&self, poly_tx_hash: H256) -> Option<Transaction> {
        if !self.is_empty_tx_by_hash(&poly_tx_hash) {
            Some(self.get_tx_by_hash(&poly_tx_hash))
        } else {
            None
        }
    }

    #[endpoint(processCrossChainTx)]
    fn process_cross_chain_tx(
        &self,
        from_chain_id: u64,
        height: u32,
        tx: Transaction,
        token_identifier: BoxedBytes,
        amount: BigUint,
    ) -> SCResult<()> {
        require!(
            !self.is_empty_token_management_contract_address(),
            "token management contract address not set"
        );

        require!(
            self.get_own_chain_id() == tx.to_chain_id,
            "This transaction is meant for another chain"
        );

        require!(
            tx.hash == self.hash_transaction(&tx),
            "Wrong transaction hash"
        );

        require!(
            self.is_empty_tx_by_hash(&tx.hash),
            "This transaction was already processed"
        );

        if !token_identifier.is_empty() && amount > 0 {
            let token_whitelist = self.get_token_whitelist();

            require!(
                token_whitelist.contains(&token_identifier),
                "Token is not on whitelist. Transaction rejected"
            );
        }

        let contract_address = self.get_header_sync_contract_address();
        let proxy = contract_proxy!(self, &contract_address, BlockHeaderSync);
        proxy.getHeaderByHeight(from_chain_id, height, &tx, &token_identifier, &amount);

        Ok(())
    }

    #[endpoint(processPendingTx)]
    fn process_pending_tx(&self, poly_tx_hash: H256) -> SCResult<()> {
        require!(
            self.get_tx_status(&poly_tx_hash) == TransactionStatus::Pending,
            "Transaction is not in Pending status"
        );

        self.process_tx(&poly_tx_hash)
    }

    #[endpoint(retryOutOfFundsTx)]
    fn retry_out_of_funds_tx(&self, poly_tx_hash: H256) -> SCResult<()> {
        require!(
            self.get_tx_status(&poly_tx_hash) == TransactionStatus::OutOfFunds,
            "Transaction is not in OutOfFunds status"
        );

        self.process_tx(&poly_tx_hash)
    }

    // callbacks

    #[callback]
    fn get_header_by_height_callback(
        &self,
        result: AsyncCallResult<Option<Header>>,
        #[callback_arg] tx: Transaction,
        #[callback_arg] token_identifier: BoxedBytes,
        #[callback_arg] amount: BigUint,
    ) {
        match result {
            AsyncCallResult::Ok(opt_header) => {
                match opt_header {
                    Some(_header) => {
                        // if this is not empty, it means processCrossChainTx was called more than once with the same tx
                        // so this should not be executed again
                        if !self.is_empty_tx_by_hash(&tx.hash) {
                            return;
                        }

                        // TODO: check tx proof

                        self.set_tx_by_hash(&tx.hash, &tx);
                        self.set_payment_for_tx(&tx.hash, &(token_identifier, amount));
                        self.set_tx_status(&tx.hash, TransactionStatus::Pending);
                    }
                    None => {
                        // could not find header
                        // should sync header first
                    }
                };
            }
            AsyncCallResult::Err(_) => {}
        }
    }

    // private

    fn hash_transaction(&self, tx: &Transaction) -> H256 {
        self.sha256(tx.get_partial_serialized().as_slice())
    }

    fn get_esdt_token_identifier_boxed(&self) -> BoxedBytes {
        BoxedBytes::from(self.get_esdt_token_name())
    }

    // deduplicates logic from ProcessPendingTx and RetryOutOfFundsTx
    // don't need chain id, as these transactions are meant for our chain, so we use own_chain_id
    fn process_tx(&self, poly_tx_hash: &H256) -> SCResult<()> {
        require!(
            !self.is_empty_token_management_contract_address(),
            "token management contract address not set"
        );
        require!(
            !self.is_empty_tx_by_hash(poly_tx_hash),
            "Transaction does not exist"
        );

        let tx = self.get_tx_by_hash(poly_tx_hash);

        // this should never fail, but we'll check just in case
        require!(&tx.hash == poly_tx_hash, "Wrong transaction hash");

        let (token_identifier, amount) = self.get_payment_for_tx(poly_tx_hash);
        let token_management_contract_address = self.get_token_management_contract_address();

        // simple transfer
        if tx.method_name.is_empty() {
            let mut serializer = HexCallDataSerializer::new(TRANSFER_ESDT_TO_ACCOUNT_ENDPOINT_NAME);
            serializer.push_argument_bytes(token_identifier.as_slice());
            serializer.push_argument_bytes(&amount.to_bytes_be());
            serializer.push_argument_bytes(tx.to_contract_address.as_bytes());
            serializer.push_argument_bytes(tx.hash.as_bytes());

            self.set_tx_status(&tx.hash, TransactionStatus::InProgress);

            self.send_tx(
                &token_management_contract_address,
                &BigUint::zero(),
                serializer.as_slice(),
            );
        }
        // scCall
        else {
            let mut serializer =
                HexCallDataSerializer::new(TRANSFER_ESDT_TO_CONTRACT_ENDPOINT_NAME);
            serializer.push_argument_bytes(token_identifier.as_slice());
            serializer.push_argument_bytes(&amount.to_bytes_be());
            serializer.push_argument_bytes(tx.to_contract_address.as_bytes());
            serializer.push_argument_bytes(tx.hash.as_bytes());

            serializer.push_argument_bytes(tx.method_name.as_slice());
            for arg in &tx.method_args {
                serializer.push_argument_bytes(arg.as_slice());
            }

            self.set_tx_status(&tx.hash, TransactionStatus::InProgress);

            self.send_tx(
                &token_management_contract_address,
                &BigUint::zero(),
                serializer.as_slice(),
            );
        }

        Ok(())
    }

    // events

    #[event("0x1000000000000000000000000000000000000000000000000000000000000001")]
    fn create_tx_event(&self, tx: &Transaction);

    // storage

    // header sync contract address

    #[storage_get("headerSyncContractAddress")]
    fn get_header_sync_contract_address(&self) -> Address;

    #[storage_set("headerSyncContractAddress")]
    fn set_header_sync_contract_address(&self, address: &Address);

    // Token management contract. Currently, this is the esdt contract
    #[storage_get("tokenManagementContractAddress")]
    fn get_token_management_contract_address(&self) -> Address;

    #[storage_set("tokenManagementContractAddress")]
    fn set_token_management_contract_address(&self, address: &Address);

    #[storage_is_empty("tokenManagementContractAddress")]
    fn is_empty_token_management_contract_address(&self) -> bool;

    // payment for a specific transaction

    #[view(getPaymentForTx)]
    #[storage_get("paymentForTx")]
    fn get_payment_for_tx(&self, poly_tx_hash: &H256) -> (BoxedBytes, BigUint);

    #[storage_set("paymentForTx")]
    fn set_payment_for_tx(
        &self,
        poly_tx_hash: &H256,
        token_identifier_amount_pair: &(BoxedBytes, BigUint),
    );

    // own chain id

    #[view(getOwnChainId)]
    #[storage_get("ownChainId")]
    fn get_own_chain_id(&self) -> u64;

    #[storage_set("ownChainId")]
    fn set_own_chain_id(&self, own_chain_id: u64);

    // cross chain tx id

    #[storage_get("crossChainTxId")]
    fn get_cross_chain_tx_id(&self, chain_id: u64) -> u64;

    #[storage_set("crossChainTxId")]
    fn set_cross_chain_tx_id(&self, chain_id: u64, tx_id: u64);

    // tx by hash

    #[storage_get("txByHash")]
    fn get_tx_by_hash(&self, poly_tx_hash: &H256) -> Transaction;

    #[storage_set("txByHash")]
    fn set_tx_by_hash(&self, poly_tx_hash: &H256, tx: &Transaction);

    #[storage_is_empty("txByHash")]
    fn is_empty_tx_by_hash(&self, poly_tx_hash: &H256) -> bool;

    // transaction status

    #[view(getTxStatus)]
    #[storage_get("txStatus")]
    fn get_tx_status(&self, poly_tx_hash: &H256) -> TransactionStatus;

    #[storage_set("txStatus")]
    fn set_tx_status(&self, poly_tx_hash: &H256, status: TransactionStatus);

    // Token whitelist

    #[storage_get("tokenWhitelist")]
    fn get_token_whitelist(&self) -> Vec<BoxedBytes>;

    #[storage_set("tokenWhitelist")]
    fn set_token_whitelist(&self, token_whitelist: &Vec<BoxedBytes>);
}
