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

#[elrond_wasm_derive::callable(SimpleEsdtProxy)]
pub trait SimpleEsdt {
    #[rustfmt::skip]
	#[callback(get_tx_status_callback)]
    fn getTxStatus(&self, tx_hash: &H256,
        #[callback_arg] tx_id: u64
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

    // endpoints

    // TODO: Accept eGLD as payment as well, and automatically wrap it if that's the case
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

        if !token_identifier.is_empty() && esdt_value > 0 {
            let token_whitelist = self.get_token_whitelist();

            require!(
                token_whitelist.contains(&token_identifier),
                "Token is not on whitelist. Transaction rejected"
            );

            self.set_payment_for_tx(to_chain_id, tx_id, &(token_identifier, esdt_value));
        }

        let from_contract_address = self.get_caller();

        let mut tx = Transaction {
            tx_hash: H256::zero(),
            tx_id,
            from_contract_address,
            to_chain_id,
            to_contract_address,
            method_name,
            method_args,
        };
        tx.tx_hash = self.hash_transaction(&tx);

        self.set_tx_by_id(to_chain_id, tx_id, &tx);
        // TODO: Add a way to mark these as Executed/Rejected by an approved address
        self.set_tx_status(to_chain_id, tx_id, TransactionStatus::Pending);
        self.set_cross_chain_tx_id(to_chain_id, tx_id + 1);

        self.create_tx_event(&tx);

        Ok(())
    }

    #[endpoint(getTxById)]
    fn get_tx_by_id_or_none(&self, chain_id: u64, tx_id: u64) -> Option<Transaction> {
        if !self.is_empty_tx_by_id(chain_id, tx_id) {
            Some(self.get_tx_by_id(chain_id, tx_id))
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
            self.is_empty_tx_by_id(tx.to_chain_id, tx.tx_id),
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
    fn process_pending_tx(&self, tx_id: u64) -> SCResult<()> {
        self.process_tx(tx_id, TransactionStatus::Pending)
    }

    #[endpoint(retryOutOfFundsTx)]
    fn retry_out_of_funds_tx(&self, tx_id: u64) -> SCResult<()> {
        self.process_tx(tx_id, TransactionStatus::OutOfFunds)
    }

    #[endpoint(completeTx)]
    fn complete_tx(&self, tx_id: u64) -> SCResult<()> {
        require!(
            !self.is_empty_token_management_contract_address(),
            "token management contract address not set"
        );

        let chain_id = self.get_own_chain_id();

        require!(
            !self.is_empty_tx_by_id(chain_id, tx_id),
            "Transaction does not exist"
        );

        require!(
            self.get_tx_status(chain_id, tx_id) == TransactionStatus::InProgress,
            "Transaction must be processed as Pending first"
        );

        let tx_hash = self.get_tx_by_id(chain_id, tx_id).tx_hash;

        let token_management_contract_address = self.get_token_management_contract_address();
        let proxy = contract_proxy!(self, &token_management_contract_address, SimpleEsdt);
        proxy.getTxStatus(&tx_hash, tx_id);

        Ok(())
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
                        if !self.is_empty_tx_by_id(tx.to_chain_id, tx.tx_id) {
                            return;
                        }

                        // TODO: check tx proof

                        let chain_id = tx.to_chain_id;
                        let tx_id = tx.tx_id;

                        self.set_tx_by_id(chain_id, tx_id, &tx);
                        self.set_payment_for_tx(chain_id, tx_id, &(token_identifier, amount));
                        self.set_tx_status(chain_id, tx_id, TransactionStatus::Pending);
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

    #[callback]
    fn get_tx_status_callback(
        &self,
        result: AsyncCallResult<TransactionStatus>,
        #[callback_arg] tx_id: u64,
    ) {
        match result {
            AsyncCallResult::Ok(tx_status) => {
                match tx_status {
                    TransactionStatus::Executed | TransactionStatus::Rejected | TransactionStatus::OutOfFunds => {
                        self.set_tx_status(self.get_own_chain_id(), tx_id, tx_status);
                    },
                    _ => {}
                } 
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
    fn process_tx(&self, tx_id: u64, tx_status: TransactionStatus) -> SCResult<()> {
        require!(
            !self.is_empty_token_management_contract_address(),
            "token management contract address not set"
        );

        let chain_id = self.get_own_chain_id();

        require!(
            !self.is_empty_tx_by_id(chain_id, tx_id),
            "Transaction does not exist"
        );
        require!(
            self.get_tx_status(chain_id, tx_id) == tx_status,
            "Transaction is not in the required status"
        );

        let tx = self.get_tx_by_id(chain_id, tx_id);
        let (token_identifier, amount) = self.get_payment_for_tx(chain_id, tx_id);
        let token_management_contract_address = self.get_token_management_contract_address();

        // simple transfer
        if tx.method_name.is_empty() {
            let mut serializer = HexCallDataSerializer::new(TRANSFER_ESDT_TO_ACCOUNT_ENDPOINT_NAME);
            serializer.push_argument_bytes(token_identifier.as_slice());
            serializer.push_argument_bytes(&amount.to_bytes_be());
            serializer.push_argument_bytes(tx.to_contract_address.as_bytes());
            serializer.push_argument_bytes(tx.tx_hash.as_bytes());

            self.set_tx_status(chain_id, tx_id, TransactionStatus::InProgress);

            self.async_call(
                &token_management_contract_address,
                &BigUint::zero(),
                serializer.as_slice(),
            );
        }
        // scCall
        else {
            let mut method_args_encoded = Vec::new();
            if !tx.method_args.is_empty() {
                method_args_encoded =
                    match elrond_wasm::elrond_codec::top_encode_to_vec(&tx.method_args) {
                        core::result::Result::Ok(encoded) => encoded,
                        core::result::Result::Err(_) => {
                            return sc_error!("failed to encode method arguments")
                        }
                    }
            }

            let mut arg_buffer = ArgBuffer::new();
            arg_buffer.push_raw_arg(token_identifier.as_slice());
            arg_buffer.push_raw_arg(&amount.to_bytes_be());
            arg_buffer.push_raw_arg(tx.to_contract_address.as_bytes());

            arg_buffer.push_raw_arg(tx.method_name.as_slice());
            arg_buffer.push_raw_arg(method_args_encoded.as_slice());

            arg_buffer.push_raw_arg(tx.tx_hash.as_bytes());

            self.set_tx_status(chain_id, tx_id, TransactionStatus::InProgress);

            self.execute_on_dest_context(
                self.get_gas_left(),
                &token_management_contract_address,
                &BigUint::zero(),
                TRANSFER_ESDT_TO_CONTRACT_ENDPOINT_NAME,
                &arg_buffer,
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
    fn get_payment_for_tx(&self, chain_id: u64, tx_id: u64) -> (BoxedBytes, BigUint);

    #[storage_set("paymentForTx")]
    fn set_payment_for_tx(
        &self,
        chain_id: u64,
        tx_id: u64,
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

    // tx by id

    #[storage_get("txById")]
    fn get_tx_by_id(&self, chain_id: u64, tx_id: u64) -> Transaction;

    #[storage_set("txById")]
    fn set_tx_by_id(&self, chain_id: u64, tx_id: u64, tx: &Transaction);

    #[storage_is_empty("txById")]
    fn is_empty_tx_by_id(&self, chain_id: u64, tx_id: u64) -> bool;

    // transaction status

    #[view(getTxStatus)]
    #[storage_get("txStatus")]
    fn get_tx_status(&self, chain_id: u64, tx_id: u64) -> TransactionStatus;

    #[storage_set("txStatus")]
    fn set_tx_status(&self, chain_id: u64, tx_id: u64, status: TransactionStatus);

    // Token whitelist

    #[storage_get("tokenWhitelist")]
    fn get_token_whitelist(&self) -> Vec<BoxedBytes>;

    #[storage_set("tokenWhitelist")]
    fn set_token_whitelist(&self, token_whitelist: &Vec<BoxedBytes>);
}
