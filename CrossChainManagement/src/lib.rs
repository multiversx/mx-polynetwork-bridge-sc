#![no_std]

use elrond_wasm::{imports, only_owner, ArgBuffer, HexCallDataSerializer};
use esdt_payment::*;
use header::*;
use transaction::*;

imports!();

const TRANSFER_ESDT_ENDPOINT_NAME: &[u8] = b"transferEsdt";

const ESDT_TRANSFER_STRING: &[u8] = b"ESDTTransfer";
const ESDT_BURN_STRING: &[u8] = b"ESDTBurn";

// erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u
const ESDT_SYSTEM_SC_ADDRESS_ARRAY: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xff,
];

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

            self.set_token_whitelist(&token_whitelist);
        }

        Ok(())
    }

    #[endpoint(removeTokenFromWhitelist)]
    fn remove_token_from_whitelist(&self, token_identifier: BoxedBytes) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        let mut token_whitelist = self.get_token_whitelist();

        for i in 0..token_whitelist.len() {
            if token_whitelist[i] == token_identifier {
                token_whitelist.remove(i);

                self.set_token_whitelist(&token_whitelist);

                break;
            }
        }

        Ok(())
    }

    #[endpoint(addAddressToApprovedlist)]
    fn add_address_to_approved_list(&self, approved_address: Address) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        let mut approved_address_list = self.get_approved_address_list();

        if !approved_address_list.contains(&approved_address) {
            approved_address_list.push(approved_address);

            self.set_approved_address_list(&approved_address_list);
        }

        Ok(())
    }

    #[endpoint(removeAddressFromApprovedlist)]
    fn remove_address_from_approved_list(&self, approved_address: Address) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        let mut approved_address_list = self.get_approved_address_list();

        for i in 0..approved_address_list.len() {
            if approved_address_list[i] == approved_address {
                approved_address_list.remove(i);

                self.set_approved_address_list(&approved_address_list);

                break;
            }
        }

        Ok(())
    }

    #[endpoint(burnTokens)]
    fn burn_tokens(&self, token_identifier: BoxedBytes) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        let mut burn_pool_token_identifier_list = self.get_burn_pool_token_identifiers();
        match burn_pool_token_identifier_list
            .iter()
            .position(|ident| ident == &token_identifier)
        {
            Some(index) => {
                burn_pool_token_identifier_list.remove(index);
            }
            None => return sc_error!("token is not in burn list"),
        };

        let amount = self.get_burn_amount_for_token(&token_identifier);

        self.set_burn_amount_for_token(&token_identifier, &BigUint::zero());

        self.burn_esdt_token(&token_identifier, &amount);

        Ok(())
    }

    #[endpoint(refundTokens)]
    fn refund_tokens(&self, token_identifier: BoxedBytes, refund_address: Address) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        let mut refund_pool_address_list = self.get_refund_pool_address_list();
        match refund_pool_address_list
            .iter()
            .position(|addr| addr == &refund_address)
        {
            Some(addr_index) => {
                let mut refund_pool_tokens_list_for_address =
                    self.get_refund_pool_tokens_list_for_address(&refund_address);

                match refund_pool_tokens_list_for_address
                    .iter()
                    .position(|ident| ident == &token_identifier)
                {
                    Some(ident_index) => {
                        refund_pool_tokens_list_for_address.remove(ident_index);

                        // if this was the last token for this address, then we remove the address from the whole list
                        if refund_pool_tokens_list_for_address.is_empty() {
                            refund_pool_address_list.remove(addr_index);

                            self.set_refund_pool_address_list(&refund_pool_address_list);
                        }

                        self.set_refund_pool_tokens_list_for_address(
                            &refund_address,
                            &refund_pool_tokens_list_for_address,
                        );
                    }
                    None => return sc_error!("token is not on the address' refund list"),
                }
            }
            None => return sc_error!("address is not on refund list"),
        }

        let refund_amount =
            self.get_refund_amount_for_token_for_address(&token_identifier, &refund_address);

        self.set_refund_amount_for_token_for_address(
            &token_identifier,
            &refund_address,
            &BigUint::zero(),
        );

        self.refund_esdt_token(&token_identifier, &refund_address, &refund_amount);

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

    // endpoints - approved addresses only

    #[endpoint(setOffchainTxStatus)]
    fn set_offchain_tx_status(
        &self,
        poly_tx_hash: H256,
        tx_status: TransactionStatus,
    ) -> SCResult<()> {
        let approved_address_list = self.get_approved_address_list();
        require!(
            approved_address_list.contains(&self.get_caller()),
            "Caller is not an approved address"
        );

        require!(
            !self.is_empty_tx_by_hash(&poly_tx_hash),
            "Transaction does not exist"
        );
        require!(
            self.get_tx_status(&poly_tx_hash) == TransactionStatus::Pending,
            "Transaction must be in Pending status"
        );

        if tx_status == TransactionStatus::Executed {
            self.add_tx_payment_to_burn_list(&poly_tx_hash);
        } else if tx_status == TransactionStatus::Rejected {
            self.add_tx_payment_to_refund_list(&poly_tx_hash);
        } else {
            return sc_error!("Transaction status may only be set to Executed or Rejected");
        }

        self.set_tx_status(&poly_tx_hash, tx_status);

        Ok(())
    }

    // endpoints

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
        require!(
            to_chain_id != self.get_own_chain_id(),
            "Must send to a chain other than Elrond"
        );

        let token_identifier = self.get_esdt_token_identifier_boxed();
        let esdt_value = self.get_esdt_value_big_uint();
        let tx_id = self.get_cross_chain_tx_id(to_chain_id);

        let from_contract_address = self.get_caller();

        let mut tx = Transaction {
            hash: H256::zero(),
            id: tx_id,
            from_contract_address: from_contract_address.clone(),
            to_chain_id,
            to_contract_address: to_contract_address.clone(),
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

            self.set_payment_for_tx(
                &tx.hash,
                &EsdtPayment {
                    sender: from_contract_address,
                    receiver: to_contract_address,
                    token_identifier,
                    amount: esdt_value,
                },
            );
        }

        self.set_tx_by_hash(&tx.hash, &tx);
        self.set_tx_status(&tx.hash, TransactionStatus::Pending);
        self.save_tx_to_pending_list(&tx.hash);
        self.set_cross_chain_tx_id(to_chain_id, tx_id + 1);

        self.create_tx_event(&tx);

        Ok(())
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

    // views

    #[view(getTxByHash)]
    fn get_tx_by_hash_or_none(&self, poly_tx_hash: H256) -> Option<Transaction> {
        if !self.is_empty_tx_by_hash(&poly_tx_hash) {
            Some(self.get_tx_by_hash(&poly_tx_hash))
        } else {
            None
        }
    }

    #[endpoint(getNextPendingCrossChainTx)]
    fn get_next_pending_cross_chain_tx() -> Option<Transaction> {
        let list_len = self.get_pending_cross_chain_tx_length();
        let current_index = self.get_pending_cross_chain_tx_current_index();

        if current_index < list_len {
            let poly_tx_hash = self.get_pending_cross_chain_tx(current_index);

            self.set_pending_cross_chain_tx_current_index(current_index + 1);

            Some(self.get_tx_by_hash(&poly_tx_hash))
        } else {
            None
        }
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
                        self.set_tx_status(&tx.hash, TransactionStatus::Pending);

                        if !token_identifier.is_empty() && amount > 0 {
                            self.set_payment_for_tx(
                                &tx.hash,
                                &EsdtPayment {
                                    sender: tx.from_contract_address,
                                    receiver: tx.to_contract_address,
                                    token_identifier,
                                    amount,
                                },
                            );
                        }
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

    fn save_tx_to_pending_list(&self, poly_tx_hash: &H256) {
        let new_tx_index = self.get_pending_cross_chain_tx_length();

        self.set_pending_cross_chain_tx(new_tx_index, poly_tx_hash);
        self.set_pending_cross_chain_tx_length(new_tx_index + 1);
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

        let esdt_payment = self.get_payment_for_tx(poly_tx_hash);
        let token_management_contract_address = self.get_token_management_contract_address();

        let mut serializer = HexCallDataSerializer::new(TRANSFER_ESDT_ENDPOINT_NAME);
        serializer.push_argument_bytes(esdt_payment.token_identifier.as_slice());
        serializer.push_argument_bytes(esdt_payment.amount.to_bytes_be().as_slice());
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

        Ok(())
    }

    fn add_tx_payment_to_burn_list(&self, poly_tx_hash: &H256) {
        if self.is_empty_payment_for_tx(poly_tx_hash) {
            return;
        }

        let esdt_payment = self.get_payment_for_tx(poly_tx_hash);
        let mut current_burn_amount =
            self.get_burn_amount_for_token(&esdt_payment.token_identifier);

        if current_burn_amount == 0 {
            let mut burn_pool_token_identifiers_list = self.get_burn_pool_token_identifiers();

            burn_pool_token_identifiers_list.push(esdt_payment.token_identifier.clone());

            self.set_burn_pool_token_identifiers(&burn_pool_token_identifiers_list);
        }

        current_burn_amount += esdt_payment.amount;

        self.set_burn_amount_for_token(&esdt_payment.token_identifier, &current_burn_amount);

        self.clear_payment_for_tx(poly_tx_hash);
    }

    fn add_tx_payment_to_refund_list(&self, poly_tx_hash: &H256) {
        if self.is_empty_payment_for_tx(poly_tx_hash) {
            return;
        }

        let refund_address = self.get_tx_by_hash(poly_tx_hash).from_contract_address;
        let esdt_payment = self.get_payment_for_tx(poly_tx_hash);
        let mut current_refund_amount = self.get_refund_amount_for_token_for_address(
            &esdt_payment.token_identifier,
            &refund_address,
        );

        if current_refund_amount == 0 {
            let mut refund_pool_tokens_list =
                self.get_refund_pool_tokens_list_for_address(&refund_address);

            // if this is empty, it means this is the first refund for this address, so we add it to the address list
            if refund_pool_tokens_list.is_empty() {
                let mut refund_pool_address_list = self.get_refund_pool_address_list();

                refund_pool_address_list.push(refund_address.clone());

                self.set_refund_pool_address_list(&refund_pool_address_list);
            }

            refund_pool_tokens_list.push(esdt_payment.token_identifier.clone());

            self.set_refund_pool_tokens_list_for_address(&refund_address, &refund_pool_tokens_list);
        }

        current_refund_amount += esdt_payment.amount;

        self.set_refund_amount_for_token_for_address(
            &esdt_payment.token_identifier,
            &refund_address,
            &current_refund_amount,
        );

        self.clear_payment_for_tx(poly_tx_hash);
    }

    fn burn_esdt_token(&self, token_identifier: &BoxedBytes, amount: &BigUint) {
        let mut serializer = HexCallDataSerializer::new(ESDT_BURN_STRING);
        serializer.push_argument_bytes(token_identifier.as_slice());
        serializer.push_argument_bytes(&amount.to_bytes_be());

        self.async_call(
            &Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY),
            &BigUint::zero(),
            serializer.as_slice(),
        );
    }

    fn refund_esdt_token(
        &self,
        token_identifier: &BoxedBytes,
        refund_address: &Address,
        amount: &BigUint,
    ) {
        let mut serializer = HexCallDataSerializer::new(ESDT_TRANSFER_STRING);
        serializer.push_argument_bytes(token_identifier.as_slice());
        serializer.push_argument_bytes(&amount.to_bytes_be());

        self.send_tx(refund_address, &BigUint::zero(), serializer.as_slice());
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

    // payment for a specific transaction - (token_identifier, amount) pair

    #[view(getPaymentForTx)]
    #[storage_get("paymentForTx")]
    fn get_payment_for_tx(&self, poly_tx_hash: &H256) -> EsdtPayment<BigUint>;

    #[storage_set("paymentForTx")]
    fn set_payment_for_tx(&self, poly_tx_hash: &H256, esdt_payment: &EsdtPayment<BigUint>);

    #[storage_clear("paymentForTx")]
    fn clear_payment_for_tx(&self, poly_tx_hash: &H256);

    #[storage_is_empty("paymentForTx")]
    fn is_empty_payment_for_tx(&self, poly_tx_hash: &H256) -> bool;

    // burn pool - vec of token names, then in a separate storage key we store the amount
    // this makes it easier to search for one specific burn token amount and update it

    #[view(getBurnPoolTokenIdentifiers)]
    #[storage_get("burnPoolTokenIdentifiers")]
    fn get_burn_pool_token_identifiers(&self) -> Vec<BoxedBytes>;

    #[storage_set("burnPoolTokenIdentifiers")]
    fn set_burn_pool_token_identifiers(&self, burn_pool: &[BoxedBytes]);

    #[view(getBurnAmountForToken)]
    #[storage_get("burnAmountForToken")]
    fn get_burn_amount_for_token(&self, token_identifier: &BoxedBytes) -> BigUint;

    #[storage_set("burnAmountForToken")]
    fn set_burn_amount_for_token(&self, token_identifier: &BoxedBytes, amount: &BigUint);

    // refund pool - split into 3 mappings
    // first, a list of all the addresses that are due for a refund
    // second, a list of all the tokens for each address
    // and last, the amount for each token.
    // This might seem overkill, but this makes it very easy to modify one specific entry instead of searching through arrays

    #[view(getRefundPoolAddressList)]
    #[storage_get("refundPoolAddressList")]
    fn get_refund_pool_address_list(&self) -> Vec<Address>;

    #[storage_set("refundPoolAddressList")]
    fn set_refund_pool_address_list(&self, address_list: &[Address]);

    #[view(getRefundPoolTokensListForAddress)]
    #[storage_get("refundPoolTokensListForAddress")]
    fn get_refund_pool_tokens_list_for_address(&self, refund_address: &Address) -> Vec<BoxedBytes>;

    #[storage_set("refundPoolTokensListForAddress")]
    fn set_refund_pool_tokens_list_for_address(
        &self,
        refund_address: &Address,
        token_identifier_list: &[BoxedBytes],
    );

    #[view(getRefundAmountForTokenForAddress)]
    #[storage_get("refundAmountForTokenForAddress")]
    fn get_refund_amount_for_token_for_address(
        &self,
        token_identifier: &BoxedBytes,
        address: &Address,
    ) -> BigUint;

    #[storage_set("refundAmountForTokenForAddress")]
    fn set_refund_amount_for_token_for_address(
        &self,
        token_identifier: &BoxedBytes,
        address: &Address,
        amount: &BigUint,
    );

    // own chain id

    #[view(getOwnChainId)]
    #[storage_get("ownChainId")]
    fn get_own_chain_id(&self) -> u64;

    #[storage_set("ownChainId")]
    fn set_own_chain_id(&self, own_chain_id: u64);

    // cross chain tx id

    #[view(getCrossChainTxId)]
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

    // list of hashes for pending tx from elrond to another chain

    #[storage_get("pendingCrosschainTxList")]
    fn get_pending_cross_chain_tx(&self, index: usize) -> H256;

    #[storage_set("pendingCrosschainTxList")]
    fn set_pending_cross_chain_tx(&self, index: usize, poly_tx_hash: &H256);

    #[storage_get("pendingCrossChainTxListLength")]
    fn get_pending_cross_chain_tx_length(&self) -> usize;

    #[storage_set("pendingCrosschainTxListLength")]
    fn set_pending_cross_chain_tx_length(&self, length: usize);

    #[storage_get("pendingCrosschainTxCurrentIndex")]
    fn get_pending_cross_chain_tx_current_index(&self) -> usize;

    #[storage_set("pendingCrosschainTxCurrentIndex")]
    fn set_pending_cross_chain_tx_current_index(&self, current_index: usize);

    // transaction status

    #[view(getTxStatus)]
    #[storage_get("txStatus")]
    fn get_tx_status(&self, poly_tx_hash: &H256) -> TransactionStatus;

    #[storage_set("txStatus")]
    fn set_tx_status(&self, poly_tx_hash: &H256, status: TransactionStatus);

    // Token whitelist

    #[view(getTokenWhitelist)]
    #[storage_get("tokenWhitelist")]
    fn get_token_whitelist(&self) -> Vec<BoxedBytes>;

    #[storage_set("tokenWhitelist")]
    fn set_token_whitelist(&self, token_whitelist: &[BoxedBytes]);

    // Approved address list - These addresses can mark transactions as executed/rejected and trigger a burn/refund respectively

    #[view(getApprovedAddressList)]
    #[storage_get("approvedAddressList")]
    fn get_approved_address_list(&self) -> Vec<Address>;

    #[storage_set("approvedAddressList")]
    fn set_approved_address_list(&self, approved_address_list: &[Address]);
}
