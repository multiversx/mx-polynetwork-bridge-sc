#![no_std]

use elrond_wasm::{imports, only_owner, HexCallDataSerializer};
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
        #[callback_arg] token_identifier: &TokenIdentifier,
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
    fn add_token_to_whitelist(&self, token_identifier: TokenIdentifier) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        if self.get_token_whitelist_mapper().insert(token_identifier) {
            Ok(())
        }
        else {
            sc_error!("Token was already in whitelist")
        }
    }

    #[endpoint(removeTokenFromWhitelist)]
    fn remove_token_from_whitelist(&self, token_identifier: TokenIdentifier) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        if self.get_token_whitelist_mapper().remove(&token_identifier) {
            Ok(())
        }
        else {
            sc_error!("Token was not in whitelist")
        }
    }

    #[endpoint(addAddressToApprovedlist)]
    fn add_address_to_approved_list(&self, approved_address: Address) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        if self.get_approved_address_list_mapper().insert(approved_address) {
            Ok(())
        }
        else {
            sc_error!("Address was already in the list")
        }
    }

    #[endpoint(removeAddressFromApprovedlist)]
    fn remove_address_from_approved_list(&self, approved_address: Address) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        if self.get_approved_address_list_mapper().remove(&approved_address) {
            Ok(())
        }
        else {
            sc_error!("Address was not in the list")
        }
    }

    #[endpoint(burnTokens)]
    fn burn_tokens(&self, token_identifier: TokenIdentifier) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        let mut burn_amount_for_token_mapper = self.get_burn_amount_mapper();

        match burn_amount_for_token_mapper.get(&token_identifier) {
            Some(amount) => {
                burn_amount_for_token_mapper.remove(&token_identifier);

                self.burn_esdt_token(&token_identifier, &amount);

                Ok(())
            }
            None => sc_error!("token is not in burn list"),
        }
    }

    #[endpoint(refundTokens)]
    fn refund_tokens(
        &self,
        token_identifier: TokenIdentifier,
        refund_address: Address,
    ) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        let mut refund_amount_mapper = self.get_refund_amount_mapper(&refund_address);
        let refund_amount = match refund_amount_mapper.get(&token_identifier) {
            Some(amount) => amount,
            None => return sc_error!("token is not on the address' refund list")
        };

        refund_amount_mapper.remove(&token_identifier);

        if refund_amount_mapper.is_empty() {
            self.get_refund_address_set_mapper().remove(&refund_address);
        }

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
        require!(
            self.get_approved_address_list_mapper().contains(&self.get_caller()),
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

    #[payable("*")]
    #[endpoint(createCrossChainTx)]
    fn create_cross_chain_tx(
        &self,
        to_chain_id: u64,
        to_contract_address: Address,
        method_name: BoxedBytes,
        method_args: Vec<BoxedBytes>,
        #[payment_token] token_identifier: TokenIdentifier,
        #[payment] esdt_value: BigUint,
    ) -> SCResult<()> {
        require!(
            !self.is_empty_token_management_contract_address(),
            "token management contract address not set"
        );
        require!(
            to_chain_id != self.get_own_chain_id(),
            "Must send to a chain other than Elrond"
        );

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

        if token_identifier.is_esdt() && esdt_value > 0 {
            require!(
                self.get_token_whitelist_mapper().contains(&token_identifier),
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
        self.get_pending_cross_chain_tx_list_mapper().push_back(tx.hash.clone());
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
        token_identifier: TokenIdentifier,
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

        if token_identifier.is_esdt() && amount > 0 {
            require!(
                self.get_token_whitelist_mapper().contains(&token_identifier),
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

    #[endpoint(getNextPendingCrossChainTx)]
    fn get_next_pending_cross_chain_tx() -> Option<Transaction> {
        match self.get_pending_cross_chain_tx_list_mapper().front() {
            Some(poly_tx_hash) => Some(self.get_tx_by_hash(&poly_tx_hash)),
            None => None
        }
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

    #[view(getBurnTokensList)]
    fn get_burn_tokens_list(&self) -> Vec<TokenIdentifier> {
        let mut token_list = Vec::new();

        for token_identifier in self.get_burn_amount_mapper().keys() {
            token_list.push(token_identifier);
        }

        token_list
    }

    // TO DO
    #[view(getRefundAddressList)]
    fn get_refund_address_list(&self) -> Vec<Address> {
        //self.get_refund_address_set_mapper()

        Vec::new()
    }

    // callbacks

    #[callback]
    fn get_header_by_height_callback(
        &self,
        result: AsyncCallResult<Option<Header>>,
        #[callback_arg] tx: Transaction,
        #[callback_arg] token_identifier: TokenIdentifier,
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

                        // TODO: Add transactions to a list

                        if token_identifier.is_esdt() && amount > 0 {
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

        // TODO: Replace with async_call without callback
        self.send().direct_egld(
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
        let mut burn_amount_for_token_mapper = self.get_burn_amount_mapper();

        let mut current_burn_amount =
            match burn_amount_for_token_mapper.get(&esdt_payment.token_identifier) {
                Some(amount) => amount,
                None => BigUint::zero(),
            };

        current_burn_amount += esdt_payment.amount;

        burn_amount_for_token_mapper.insert(esdt_payment.token_identifier, current_burn_amount);

        self.clear_payment_for_tx(poly_tx_hash);
    }

    fn add_tx_payment_to_refund_list(&self, poly_tx_hash: &H256) {
        if self.is_empty_payment_for_tx(poly_tx_hash) {
            return;
        }

        let refund_address = self.get_tx_by_hash(poly_tx_hash).from_contract_address;
        let esdt_payment = self.get_payment_for_tx(poly_tx_hash);

        let mut refund_amount_mapper = self.get_refund_amount_mapper(&refund_address);
        if refund_amount_mapper.is_empty() {
            self.get_refund_address_set_mapper().insert(refund_address);
        }
        
        let mut current_refund_amount = match refund_amount_mapper.get(&esdt_payment.token_identifier) {
            Some(amount) => amount,
            None => BigUint::zero()
        };

        current_refund_amount += esdt_payment.amount;

        refund_amount_mapper.insert(esdt_payment.token_identifier, current_refund_amount);

        self.clear_payment_for_tx(poly_tx_hash);
    }

    fn burn_esdt_token(&self, token_identifier: &TokenIdentifier, amount: &BigUint) {
        let mut serializer = HexCallDataSerializer::new(ESDT_BURN_STRING);
        serializer.push_argument_bytes(token_identifier.as_slice());
        serializer.push_argument_bytes(&amount.to_bytes_be());

        self.send().async_call_raw(
            &Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY),
            &BigUint::zero(),
            serializer.as_slice(),
        );
    }

    fn refund_esdt_token(
        &self,
        token_identifier: &TokenIdentifier,
        refund_address: &Address,
        amount: &BigUint,
    ) {
        let mut serializer = HexCallDataSerializer::new(ESDT_TRANSFER_STRING);
        serializer.push_argument_bytes(token_identifier.as_slice());
        serializer.push_argument_bytes(&amount.to_bytes_be());

        // TODO: Replace with send_esdt
        self.send()
            .direct_egld(refund_address, &BigUint::zero(), serializer.as_slice());
    }

    // events

    #[event("0x1000000000000000000000000000000000000000000000000000000000000001")]
    fn create_tx_event(&self, tx: &Transaction);

    // storage TODO: Use storage mappers

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

    // burn amounts for tokens

    #[storage_mapper("burnAmount")]
    fn get_burn_amount_mapper(
        &self,
    ) -> MapMapper<Self::Storage, TokenIdentifier, BigUint>;

    // refund pool

	#[storage_mapper("refundAddressSet")]
	fn get_refund_address_set_mapper(&self) -> SetMapper<Self::Storage, Address>;

    #[storage_mapper("refundAmount")]
    fn get_refund_amount_mapper(
        &self,
        address: &Address,
    ) -> MapMapper<Self::Storage, TokenIdentifier, BigUint>;

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

    #[storage_mapper("pendingCrosschainTxList")]
    fn get_pending_cross_chain_tx_list_mapper(&self) -> LinkedListMapper<Self::Storage, H256>;

    // transaction status

    #[view(getTxStatus)]
    #[storage_get("txStatus")]
    fn get_tx_status(&self, poly_tx_hash: &H256) -> TransactionStatus;

    #[storage_set("txStatus")]
    fn set_tx_status(&self, poly_tx_hash: &H256, status: TransactionStatus);

    // Token whitelist

    #[storage_mapper("tokenWhitelist")]
	fn get_token_whitelist_mapper(&self) -> SetMapper<Self::Storage, TokenIdentifier>;

    // Approved address list - These addresses can mark transactions as executed/rejected and trigger a burn/refund respectively

    #[storage_mapper("approvedAddressList")]
	fn get_approved_address_list_mapper(&self) -> SetMapper<Self::Storage, Address>;
}
