#![no_std]
#![allow(non_snake_case)]

use esdt_payment::*;
use transaction::*;

elrond_wasm::imports!();

mod token_op;

#[elrond_wasm_derive::contract]
pub trait CrossChainManagement: token_op::TokenTransferModule {
    #[init]
    fn init(&self, header_sync_contract_address: Address, own_chain_id: u64) {
        self.header_sync_contract_address()
            .set(&header_sync_contract_address);
        self.own_chain_id().set(&own_chain_id);
    }

    // endpoints - owner-only

    #[endpoint(addTokenToWhitelist)]
    fn add_token_to_whitelist(&self, token_id: TokenIdentifier) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");
        self.require_local_mint_role_set(&token_id)?;
        self.require_local_burn_role_set(&token_id)?;

        self.token_whitelist().insert(token_id);

        Ok(())
    }

    #[endpoint(removeTokenFromWhitelist)]
    fn remove_token_from_whitelist(&self, token_id: TokenIdentifier) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        self.token_whitelist().remove(&token_id);

        Ok(())
    }

    #[endpoint(addAddressToApprovedlist)]
    fn add_address_to_approved_list(&self, approved_address: Address) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        self.approved_address_list().insert(approved_address);

        Ok(())
    }

    #[endpoint(removeAddressFromApprovedlist)]
    fn remove_address_from_approved_list(&self, approved_address: Address) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        self.approved_address_list().remove(&approved_address);

        Ok(())
    }

    // endpoints - approved addresses only

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

    /// Transactions from other_chain -> Elrond
    #[endpoint(processCrossChainTx)]
    fn process_cross_chain_tx(
        &self,
        from_chain_id: u64,
        height: u32,
        tx: Transaction,
        token_id: TokenIdentifier,
        amount: Self::BigUint,
    ) -> SCResult<AsyncCall<Self::SendApi>> {
        self.require_caller_approved()?;
        require!(
            self.own_chain_id().get() == tx.to_chain_id,
            "This transaction is meant for another chain"
        );
        require!(
            tx.hash == self.hash_transaction(&tx),
            "Wrong transaction hash"
        );
        require!(
            self.tx_by_hash(&tx.hash).is_empty(),
            "This transaction was already processed"
        );
        require!(
            self.token_whitelist().contains(&token_id),
            "Token is not on whitelist. Transaction rejected"
        );
        require!(amount > 0, "Must transfer more than 0");

        let elrond_dest_address = self.try_convert_to_elrond_address(&tx.to_contract_address)?;
        if !self.blockchain().is_smart_contract(&elrond_dest_address) && !tx.method_name.is_empty()
        {
            return sc_error!("Can't call function, destination is not a smart contract");
        }

        let block_header_sync_address = self.header_sync_contract_address().get();
        let opt_header = self
            .block_header_sync_proxy(block_header_sync_address)
            .get_header_by_height_endpoint(from_chain_id, height)
            .execute_on_dest_context(self.blockchain().get_gas_left());

        match opt_header {
            OptionalResult::Some(_header) => {
                // TODO: check tx proof

                self.tx_by_hash(&tx.hash).set(&tx);
                self.tx_status(&tx.hash).set(&TransactionStatus::Pending);

                // TODO: Add transactions to a list (or is fired event enough?)
                // TODO: Decide how the tx hashes are linked to the Header
                self.receive_tx_event(&tx);

                let esdt_payment = EsdtPayment {
                    sender: tx.from_contract_address.clone(),
                    receiver: tx.to_contract_address.clone(),
                    token_id,
                    amount,
                };
                self.payment_for_tx(&tx.hash).set(&esdt_payment);

                self.process_pending_tx(tx, esdt_payment)
            }
            OptionalResult::None => {
                sc_error!("Could not find header")
            }
        }
    }

    // endpoints

    /// Transactions from Elrond -> other_chain
    #[payable("*")]
    #[endpoint(createCrossChainTx)]
    fn create_cross_chain_tx(
        &self,
        #[payment_token] token_identifier: TokenIdentifier,
        #[payment] esdt_value: Self::BigUint,
        to_chain_id: u64,
        to_contract_address: BoxedBytes,
        method_name: BoxedBytes,
        #[var_args] method_args: VarArgs<BoxedBytes>,
    ) -> SCResult<()> {
        require!(
            to_chain_id != self.own_chain_id().get(),
            "Must send to a chain other than Elrond"
        );
        require!(token_identifier.is_esdt(), "eGLD payment not allowed");
        require!(
            self.call_value().esdt_token_nonce() == 0,
            "Can't transfer NFT"
        );
        require!(esdt_value > 0, "Must transfer more than 0");

        let tx_id = self.cross_chain_tx_id(to_chain_id).get();
        let caller = self.blockchain().get_caller();
        let from_contract_address = BoxedBytes::from(caller.as_bytes());
        let mut tx = Transaction {
            hash: H256::zero(),
            id: tx_id,
            from_contract_address: from_contract_address.clone(),
            to_chain_id,
            to_contract_address: to_contract_address.clone(),
            method_name,
            method_args: method_args.into_vec(),
        };
        tx.hash = self.hash_transaction(&tx);

        if token_identifier.is_esdt() && esdt_value > 0 {
            require!(
                self.token_whitelist().contains(&token_identifier),
                "Token is not on whitelist. Transaction rejected"
            );

            self.payment_for_tx(&tx.hash).set(&EsdtPayment {
                sender: from_contract_address,
                receiver: to_contract_address,
                token_id: token_identifier,
                amount: esdt_value,
            });
        }

        self.tx_by_hash(&tx.hash).set(&tx);
        self.tx_status(&tx.hash).set(&TransactionStatus::Pending);
        self.pending_cross_chain_tx_list()
            .push_back(tx.hash.clone());
        self.cross_chain_tx_id(to_chain_id).set(&(tx_id + 1));

        self.create_tx_event(&tx);

        Ok(())
    }

    // views

    #[view(getTxByHash)]
    fn get_tx_by_hash_or_none(&self, poly_tx_hash: H256) -> OptionalResult<Transaction> {
        if !self.tx_by_hash(&poly_tx_hash).is_empty() {
            OptionalResult::Some(self.tx_by_hash(&poly_tx_hash).get())
        } else {
            OptionalResult::None
        }
    }

    // private

    fn process_pending_tx(
        &self,
        tx: Transaction,
        esdt_payment: EsdtPayment<Self::BigUint>,
    ) -> SCResult<AsyncCall<Self::SendApi>> {
        self.tx_status(&tx.hash).set(&TransactionStatus::InProgress);

        let elrond_dest_address = self.try_convert_to_elrond_address(&tx.to_contract_address)?;
        if self.blockchain().is_smart_contract(&elrond_dest_address) {
            Ok(self
                .sc_async_transfer_esdt(
                    elrond_dest_address,
                    esdt_payment.token_id,
                    esdt_payment.amount,
                    tx.method_name,
                    &tx.method_args,
                )
                .with_callback(self.callbacks().async_transfer_callback(tx.hash)))
        } else {
            Ok(self
                .account_async_transfer_esdt(
                    elrond_dest_address,
                    esdt_payment.token_id,
                    esdt_payment.amount,
                )
                .with_callback(self.callbacks().async_transfer_callback(tx.hash)))
        }
    }

    fn hash_transaction(&self, tx: &Transaction) -> H256 {
        self.crypto().sha256(tx.get_partial_serialized().as_slice())
    }

    fn try_convert_to_elrond_address(&self, address: &BoxedBytes) -> SCResult<Address> {
        require!(
            address.len() == Address::len_bytes(),
            "Wrong address format, it should be exactly 32 bytes"
        );

        Ok(Address::from_slice(address.as_slice()))
    }

    fn refund_payment_for_tx(&self, poly_tx_hash: &H256) {
        if self.payment_for_tx(poly_tx_hash).is_empty() {
            return;
        }

        let payment = self.payment_for_tx(poly_tx_hash).get();

        // this should never fail, but calling unwrap directly adds a lot of wasm bloat
        // so we check anyway
        if let Ok(elrond_dest_address) = self.try_convert_to_elrond_address(&payment.sender) {
            let _ = self.transfer_esdt(
                &elrond_dest_address,
                &payment.token_id,
                &payment.amount,
                self.data_or_empty(&elrond_dest_address, b"refund"),
            );
        }
    }

    fn try_burn_payment_for_tx(&self, poly_tx_hash: &H256) -> SCResult<()> {
        if self.payment_for_tx(poly_tx_hash).is_empty() {
            return Ok(());
        }

        let payment = self.payment_for_tx(poly_tx_hash).get();
        self.try_burn(&payment.token_id, &payment.amount)?;

        Ok(())
    }

    fn data_or_empty(&self, to: &Address, data: &'static [u8]) -> &[u8] {
        if self.blockchain().is_smart_contract(to) {
            &[]
        } else {
            data
        }
    }

    fn require_caller_approved(&self) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        require!(
            self.approved_address_list().contains(&caller),
            "Caller is not an approved address"
        );

        Ok(())
    }

    // callbacks

    #[callback]
    fn async_transfer_callback(
        &self,
        poly_tx_hash: H256,
        #[call_result] result: AsyncCallResult<()>,
    ) {
        match result {
            AsyncCallResult::Ok(()) => self
                .tx_status(&poly_tx_hash)
                .set(&TransactionStatus::Executed),
            AsyncCallResult::Err(_) => self
                .tx_status(&poly_tx_hash)
                .set(&TransactionStatus::Rejected),
        }
    }

    // proxies

    #[proxy]
    fn block_header_sync_proxy(
        &self,
        sc_address: Address,
    ) -> block_header_sync::Proxy<Self::SendApi>;

    // events

    // for tx from Elrond to another chain
    #[event("createTransaction")]
    fn create_tx_event(&self, tx: &Transaction);

    // for tx from another chain to Elrond
    #[event("receiveTransaction")]
    fn receive_tx_event(&self, tx: &Transaction);

    // storage

    #[storage_mapper("headerSyncContractAddress")]
    fn header_sync_contract_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getPaymentForTx)]
    #[storage_mapper("paymentForTx")]
    fn payment_for_tx(
        &self,
        poly_tx_hash: &H256,
    ) -> SingleValueMapper<Self::Storage, EsdtPayment<Self::BigUint>>;

    #[view(getOwnChainId)]
    #[storage_mapper("ownChainId")]
    fn own_chain_id(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[view(getCrossChainTxId)]
    #[storage_mapper("crossChainTxId")]
    fn cross_chain_tx_id(&self, chain_id: u64) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("txByHash")]
    fn tx_by_hash(&self, poly_tx_hash: &H256) -> SingleValueMapper<Self::Storage, Transaction>;

    // list of hashes for pending tx from elrond to another chain
    #[storage_mapper("pendingCrosschainTxList")]
    fn pending_cross_chain_tx_list(&self) -> LinkedListMapper<Self::Storage, H256>;

    #[view(getTxStatus)]
    #[storage_mapper("txStatus")]
    fn tx_status(&self, poly_tx_hash: &H256)
        -> SingleValueMapper<Self::Storage, TransactionStatus>;

    #[storage_mapper("tokenWhitelist")]
    fn token_whitelist(&self) -> SetMapper<Self::Storage, TokenIdentifier>;

    // Approved address list - These addresses can mark transactions as executed/rejected
    // which triggers a burn/refund respectively
    // Only for Elrond -> other_chain transactions
    #[storage_mapper("approvedAddressList")]
    fn approved_address_list(&self) -> SetMapper<Self::Storage, Address>;
}
