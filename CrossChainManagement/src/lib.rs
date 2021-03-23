#![no_std]
#![allow(non_snake_case)]

use esdt_payment::*;
use header::*;
use transaction::*;

elrond_wasm::imports!();

#[elrond_wasm_derive::callable(BlockHeaderSyncProxy)]
pub trait BlockHeaderSync {
    fn getHeaderByHeight(
        &self,
        chain_id: u64,
        height: u32,
    ) -> ContractCall<BigUint, OptionalResult<Header>>;
}

#[elrond_wasm_derive::callable(EsdtTokenManagerProxy)]
pub trait EsdtTokenManager {
    fn transferEsdt(
        &self,
        token_identifier: TokenIdentifier,
        amount: BigUint,
        to: Address,
        poly_tx_hash: H256,
        func_name: BoxedBytes,
        #[var_args] args: VarArgs<BoxedBytes>,
    ) -> ContractCall<BigUint, Option<Header>>;
}

#[elrond_wasm_derive::contract(CrossChainManagementImpl)]
pub trait CrossChainManagement {
    #[init]
    fn init(&self, header_sync_contract_address: Address, own_chain_id: u64) {
        self.header_sync_contract_address()
            .set(&header_sync_contract_address);
        self.own_chain_id().set(&own_chain_id);
    }

    // endpoints - owner-only

    #[endpoint(setTokenManagementContractAddress)]
    fn set_token_management_contract_address_endpoint(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        self.token_management_contract_address().set(&address);

        Ok(())
    }

    #[endpoint(addTokenToWhitelist)]
    fn add_token_to_whitelist(&self, token_id: TokenIdentifier) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

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

    #[endpoint(burnTokens)]
    fn burn_tokens(&self, token_id: TokenIdentifier) -> SCResult<AsyncCall<BigUint>> {
        only_owner!(self, "only owner may call this function");

        let mut burn_amount_for_token_mapper = self.burn_amounts();

        match burn_amount_for_token_mapper.get(&token_id) {
            Some(amount) => {
                burn_amount_for_token_mapper.remove(&token_id);

                Ok(self.burn_esdt_token(&token_id, &amount))
            }
            None => sc_error!("token is not in burn list"),
        }
    }

    // endpoints - token manager contract only

    #[endpoint(completeTx)]
    fn complete_tx(&self, poly_tx_hash: H256, tx_status: TransactionStatus) -> SCResult<()> {
        require!(
            !self.token_management_contract_address().is_empty(),
            "token management contract address not set"
        );
        require!(
            self.get_caller() == self.token_management_contract_address().get(),
            "Only the token manager contract may call this"
        );
        require!(
            !self.tx_by_hash(&poly_tx_hash).is_empty(),
            "Transaction does not exist"
        );
        require!(
            self.tx_status(&poly_tx_hash).get() == TransactionStatus::InProgress,
            "Transaction must be processed as Pending first"
        );
        require!(
            tx_status == TransactionStatus::Executed || tx_status == TransactionStatus::Rejected,
            "Transaction status may only be set to Executed or Rejected"
        );

        self.tx_status(&poly_tx_hash).set(&tx_status);

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
            self.approved_address_list().contains(&self.get_caller()),
            "Caller is not an approved address"
        );

        require!(
            !self.tx_by_hash(&poly_tx_hash).is_empty(),
            "Transaction does not exist"
        );
        require!(
            self.tx_status(&poly_tx_hash).get() == TransactionStatus::Pending,
            "Transaction must be in Pending status"
        );

        if tx_status == TransactionStatus::Executed {
            self.tx_status(&poly_tx_hash)
                .set(&TransactionStatus::Executed);
            self.add_tx_payment_to_burn_list(&poly_tx_hash);
        } else if tx_status == TransactionStatus::Rejected {
            self.tx_status(&poly_tx_hash)
                .set(&TransactionStatus::Rejected);
            self.refund_payment_for_tx(&poly_tx_hash);
        } else {
            return sc_error!("Transaction status may only be set to Executed or Rejected");
        }

        Ok(())
    }

    // endpoints

    // TODO: At some point, make it so eGLD is accepted and automatically wrapped
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
            !self.token_management_contract_address().is_empty(),
            "token management contract address not set"
        );
        require!(
            to_chain_id != self.own_chain_id().get(),
            "Must send to a chain other than Elrond"
        );
        require!(
            !(token_identifier.is_egld() && esdt_value > 0),
            "eGLD payment not allowed"
        );

        let tx_id = self.cross_chain_tx_id(to_chain_id).get();
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

    #[endpoint(processCrossChainTx)]
    fn process_cross_chain_tx(
        &self,
        from_chain_id: u64,
        height: u32,
        tx: Transaction,
        token_identifier: TokenIdentifier,
        amount: BigUint,
    ) -> SCResult<AsyncCall<BigUint>> {
        require!(
            !self.token_management_contract_address().is_empty(),
            "token management contract address not set"
        );
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
        if !self.is_smart_contract(&tx.to_contract_address) && !tx.method_name.is_empty() {
            return sc_error!("Can't call function, destination is not a smart contract");
        }

        if token_identifier.is_esdt() && amount > 0 {
            require!(
                self.token_whitelist().contains(&token_identifier),
                "Token is not on whitelist. Transaction rejected"
            );
        }

        let contract_address = self.header_sync_contract_address().get();

        Ok(contract_call!(self, contract_address, BlockHeaderSyncProxy)
            .getHeaderByHeight(from_chain_id, height)
            .async_call()
            .with_callback(self.callbacks().get_header_by_height_callback(
                tx,
                token_identifier,
                amount,
            )))
    }

    #[endpoint(processPendingTx)]
    fn process_pending_tx(&self, poly_tx_hash: H256) -> SCResult<TransferEgldExecute<BigUint>> {
        require!(
            self.tx_status(&poly_tx_hash).get() == TransactionStatus::Pending,
            "Transaction is not in Pending status"
        );
        require!(
            !self.token_management_contract_address().is_empty(),
            "token management contract address not set"
        );
        require!(
            !self.tx_by_hash(&poly_tx_hash).is_empty(),
            "Transaction does not exist"
        );

        let tx = self.tx_by_hash(&poly_tx_hash).get();

        // this should never fail, but we'll check just in case
        require!(tx.hash == poly_tx_hash, "Wrong poly transaction hash");

        let esdt_payment = self.payment_for_tx(&poly_tx_hash).get();
        let token_management_contract_address = self.token_management_contract_address().get();

        self.tx_status(&tx.hash).set(&TransactionStatus::InProgress);

        Ok(contract_call!(
            self,
            token_management_contract_address,
            EsdtTokenManagerProxy
        )
        .transferEsdt(
            esdt_payment.token_id,
            esdt_payment.amount,
            tx.to_contract_address,
            tx.hash,
            tx.method_name,
            VarArgs::from(tx.method_args),
        )
        .transfer_egld_execute())
    }

    #[endpoint(getNextPendingCrossChainTx)]
    fn get_next_pending_cross_chain_tx(&self) -> OptionalResult<Transaction> {
        match self.pending_cross_chain_tx_list().pop_front() {
            Some(poly_tx_hash) => OptionalResult::Some(self.tx_by_hash(&poly_tx_hash).get()),
            None => OptionalResult::None,
        }
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

    #[view(getBurnTokensList)]
    fn get_burn_tokens_list(&self) -> MultiResultVec<TokenIdentifier> {
        let mut token_list = Vec::new();

        for token_identifier in self.burn_amounts().keys() {
            token_list.push(token_identifier);
        }

        token_list.into()
    }

    // private

    fn hash_transaction(&self, tx: &Transaction) -> H256 {
        self.sha256(tx.get_partial_serialized().as_slice())
    }

    fn add_tx_payment_to_burn_list(&self, poly_tx_hash: &H256) {
        if self.payment_for_tx(poly_tx_hash).is_empty() {
            return;
        }

        let esdt_payment = self.payment_for_tx(poly_tx_hash).get();

        let mut current_burn_amount = match self.burn_amounts().get(&esdt_payment.token_id)
        {
            Some(amount) => amount,
            None => BigUint::zero(),
        };
        current_burn_amount += esdt_payment.amount;

        self.burn_amounts()
            .insert(esdt_payment.token_id, current_burn_amount);

        self.payment_for_tx(poly_tx_hash).clear();
    }

    fn burn_esdt_token(
        &self,
        token_identifier: &TokenIdentifier,
        amount: &BigUint,
    ) -> AsyncCall<BigUint> {
        ESDTSystemSmartContractProxy::new()
            .burn(token_identifier.as_esdt_identifier(), amount)
            .async_call()
    }

    fn refund_payment_for_tx(&self, poly_tx_hash: &H256) {
        if self.payment_for_tx(poly_tx_hash).is_empty() {
            return;
        }

        let payment = self.payment_for_tx(poly_tx_hash).get();

        self.send().direct_esdt_via_async_call(
            &payment.sender,
            payment.token_id.as_esdt_identifier(),
            &payment.amount,
            self.data_or_empty(&payment.sender, b"refund"),
        );
    }

    fn data_or_empty(&self, to: &Address, data: &'static [u8]) -> &[u8] {
        if self.is_smart_contract(to) {
            &[]
        } else {
            data
        }
    }

    // callbacks

    #[callback]
    fn get_header_by_height_callback(
        &self,
        #[call_result] result: AsyncCallResult<Option<Header>>,
        tx: Transaction,
        token_identifier: TokenIdentifier,
        amount: BigUint,
    ) {
        match result {
            AsyncCallResult::Ok(opt_header) => {
                match opt_header {
                    Some(_header) => {
                        // if this is not empty, it means processCrossChainTx was called more than once with the same tx
                        // so this should not be executed again
                        if !self.tx_by_hash(&tx.hash).is_empty() {
                            return;
                        }

                        // TODO: check tx proof

                        self.tx_by_hash(&tx.hash).set(&tx);
                        self.tx_status(&tx.hash).set(&TransactionStatus::Pending);

                        // TODO: Add transactions to a list (or is fired event enough?)
                        // TODO: Decide how the tx hashes are linked to the Header

                        self.receive_tx_event(&tx);

                        if token_identifier.is_esdt() && amount > 0 {
                            self.payment_for_tx(&tx.hash).set(&EsdtPayment {
                                sender: tx.from_contract_address,
                                receiver: tx.to_contract_address,
                                token_id: token_identifier,
                                amount,
                            });
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

    // events

    // for tx from Elrond to another chain
    #[event("createTransaction")]
    fn create_tx_event(&self, tx: &Transaction);

    // for tx from another chain to Elrond
    #[event("receiveTransaction")]
    fn receive_tx_event(&self, tx: &Transaction);

    // header sync contract address

    #[storage_mapper("headerSyncContractAddress")]
    fn header_sync_contract_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    // Token management contract. Currently, this is the esdt contract

    #[storage_mapper("tokenManagementContractAddress")]
    fn token_management_contract_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    // payment for a specific transaction - (token_identifier, amount) pair

    #[view(getPaymentForTx)]
    #[storage_mapper("paymentForTx")]
    fn payment_for_tx(
        &self,
        poly_tx_hash: &H256,
    ) -> SingleValueMapper<Self::Storage, EsdtPayment<BigUint>>;

    // burn amounts for tokens
    // To save gas, we only burn from time to time instead of burning for each successfully executed tx

    #[storage_mapper("burnAmounts")]
    fn burn_amounts(&self) -> MapMapper<Self::Storage, TokenIdentifier, BigUint>;

    // own chain id

    #[view(getOwnChainId)]
    #[storage_mapper("ownChainId")]
    fn own_chain_id(&self) -> SingleValueMapper<Self::Storage, u64>;

    // cross chain tx id

    #[view(getCrossChainTxId)]
    #[storage_mapper("crossChainTxId")]
    fn cross_chain_tx_id(&self, chain_id: u64) -> SingleValueMapper<Self::Storage, u64>;

    // tx by hash

    #[storage_mapper("txByHash")]
    fn tx_by_hash(&self, poly_tx_hash: &H256) -> SingleValueMapper<Self::Storage, Transaction>;

    // list of hashes for pending tx from elrond to another chain

    #[storage_mapper("pendingCrosschainTxList")]
    fn pending_cross_chain_tx_list(&self) -> LinkedListMapper<Self::Storage, H256>;

    // transaction status

    #[view(getTxStatus)]
    #[storage_mapper("txStatus")]
    fn tx_status(&self, poly_tx_hash: &H256)
        -> SingleValueMapper<Self::Storage, TransactionStatus>;

    // Token whitelist

    #[storage_mapper("tokenWhitelist")]
    fn token_whitelist(&self) -> SetMapper<Self::Storage, TokenIdentifier>;

    // Approved address list - These addresses can mark transactions as executed/rejected and trigger a burn/refund respectively

    #[storage_mapper("approvedAddressList")]
    fn approved_address_list(&self) -> SetMapper<Self::Storage, Address>;
}
