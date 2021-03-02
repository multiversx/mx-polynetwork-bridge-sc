#![no_std]
#![allow(clippy::string_lit_as_bytes)]

use transaction::TransactionStatus;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod transfer_esdt_action_result;
use transfer_esdt_action_result::*;

// erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u
const ESDT_SYSTEM_SC_ADDRESS_ARRAY: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xff,
];

const ESDT_ISSUE_COST: u64 = 5000000000000000000; // 5 eGLD

const ESDT_ISSUE_STRING: &[u8] = b"issue";
const ESDT_MINT_STRING: &[u8] = b"mint";
const ESDT_BURN_STRING: &[u8] = b"ESDTBurn";

const WRAPPED_EGLD_DISPLAY_NAME: &[u8] = b"WrappedEGLD";
const WRAPPED_EGLD_TICKER: &[u8] = b"WEGLD";
const EGLD_DECIMALS: u8 = 18;

const COMPLETE_TX_ENDPOINT_NAME: &[u8] = b"completeTx";

#[derive(TopEncode, TopDecode)]
pub enum EsdtOperation<BigUint: BigUintApi> {
    None,
    Issue,
    Mint(TokenIdentifier, BigUint), // token + amount minted
    Burn(TokenIdentifier, BigUint), // token + amount burned
}

#[elrond_wasm_derive::contract(EsdtTokenManagerImpl)]
pub trait EsdtTokenManager {
    #[init]
    fn init(&self, cross_chain_management_address: Address) {
        self.set_cross_chain_management_contract_address(&cross_chain_management_address);
    }

    // endpoints - owner-only

    #[payable("EGLD")]
    #[endpoint(performWrappedEgldIssue)]
    fn perform_wrapped_egld_issue(
        &self,
        initial_supply: BigUint,
        #[payment] payment: BigUint,
    ) -> SCResult<AsyncCall<BigUint>> {
        only_owner!(self, "only owner may call this function");

        require!(
            self.is_empty_wrapped_egld_token_identifier(),
            "wrapped egld was already issued"
        );
        require!(
            payment == BigUint::from(ESDT_ISSUE_COST),
            "Wrong payment, should pay exactly 5 eGLD for ESDT token issue"
        );

        Ok(self.issue_esdt_token(
            &BoxedBytes::from(WRAPPED_EGLD_DISPLAY_NAME),
            &BoxedBytes::from(WRAPPED_EGLD_TICKER),
            &initial_supply,
            EGLD_DECIMALS,
        ))
    }

    #[payable("EGLD")]
    #[endpoint(issueEsdtToken)]
    fn issue_esdt_token_endpoint(
        &self,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
        initial_supply: BigUint,
        num_decimals: u8,
        #[payment] payment: BigUint,
    ) -> SCResult<AsyncCall<BigUint>> {
        only_owner!(self, "only owner may call this function");

        require!(
            payment == BigUint::from(ESDT_ISSUE_COST),
            "Wrong payment, should pay exactly 5 eGLD for ESDT token issue"
        );
        require!(initial_supply > 0, "initial supply must be more than 0");

        Ok(self.issue_esdt_token(
            &token_display_name,
            &token_ticker,
            &initial_supply,
            num_decimals,
        ))
    }

    #[endpoint(mintEsdtToken)]
    fn mint_esdt_token_endpoint(
        &self,
        token_identifier: TokenIdentifier,
        amount: BigUint,
    ) -> SCResult<AsyncCall<BigUint>> {
        only_owner!(self, "only owner may call this function");

        require!(
            self.was_token_issued(&token_identifier),
            "Token must be issued first"
        );
        require!(amount > 0, "Amount minted must be more than 0");

        Ok(self.mint_esdt_token(&token_identifier, &amount))
    }

    #[endpoint(burnEsdtToken)]
    fn burn_esdt_token_endpoint(
        &self,
        token_identifier: TokenIdentifier,
        amount: BigUint,
    ) -> SCResult<AsyncCall<BigUint>> {
        only_owner!(self, "only owner may call this function");

        require!(amount > 0, "Amount burned must be more than 0");
        require!(
            amount < self.get_total_wrapped_remaining(&token_identifier),
            "Can't burn more than total wrapped remaining"
        );

        Ok(self.burn_esdt_token(&token_identifier, &amount))
    }

    // endpoints - CrossChainManagement contract - only

    #[endpoint(transferEsdt)]
    fn transfer_esdt_endpoint(
        &self,
        token_identifier: TokenIdentifier,
        amount: BigUint,
        to: Address,
        poly_tx_hash: H256,
        func_name: BoxedBytes,
        #[var_args] args: VarArgs<BoxedBytes>,
    ) -> SCResult<TransferEsdtActionResult<BigUint>> {
        require!(
            self.get_caller() == self.get_cross_chain_management_contract_address(),
            "Only the cross chain management contract may call this function"
        );

        let total_wrapped = self.get_total_wrapped_remaining(&token_identifier);
        if total_wrapped < amount {
            Ok(TransferEsdtActionResult::TransferEgldExecute(
                self.complete_tx(&poly_tx_hash, TransactionStatus::OutOfFunds),
            ))
        } else {
            if self.is_smart_contract(&to) && !func_name.is_empty() {
                // save the poly_tx_hash to be used in the callback
                self.set_temporary_storage_poly_tx_hash(&self.get_tx_hash(), &poly_tx_hash);

                if token_identifier != self.get_wrapped_egld_token_identifier() {
                    Ok(TransferEsdtActionResult::AsyncCall(
                        self.async_transfer_esdt(
                            to,
                            token_identifier,
                            amount,
                            func_name,
                            args.as_slice(),
                        ),
                    ))
                } else {
                    // automatically unwrap before sending if the token is wrapped eGLD
                    self.add_total_wrapped(&token_identifier, &amount);

                    Ok(TransferEsdtActionResult::AsyncCall(
                        self.async_transfer_egld(to, amount, func_name, args.as_slice()),
                    ))
                }
            } else {
                if token_identifier != self.get_wrapped_egld_token_identifier() {
                    Ok(TransferEsdtActionResult::SendEsdt(SendEsdt {
                        to,
                        token_name: token_identifier.into_boxed_bytes(),
                        amount,
                        data: BoxedBytes::empty(),
                    }))
                } else {
                    Ok(TransferEsdtActionResult::SendEgld(SendEgld {
                        to,
                        amount,
                        data: BoxedBytes::empty(),
                    }))
                }
            }
        }
    }

    // endpoints

    #[payable("EGLD")]
    #[endpoint(wrapEgld)]
    fn wrap_egld(&self, #[payment] payment: BigUint) -> SCResult<SendEsdt<BigUint>> {
        require!(payment > 0, "Payment must be more than 0");
        require!(
            !self.is_empty_wrapped_egld_token_identifier(),
            "Wrapped eGLD was not issued yet"
        );

        let wrapped_egld_token_identifier = self.get_wrapped_egld_token_identifier();
        let wrapped_egld_left = self.get_total_wrapped_remaining(&wrapped_egld_token_identifier);

        require!(
            wrapped_egld_left >= payment,
            "Contract does not have enough wrapped eGLD. Please try again once more is minted."
        );

        self.subtract_total_wrapped(&wrapped_egld_token_identifier, &payment);

        Ok(SendEsdt {
            to: self.get_caller(),
            token_name: wrapped_egld_token_identifier.into_boxed_bytes(),
            amount: payment,
            data: BoxedBytes::from(&b"wrapping"[..]),
        })
    }

    #[payable("*")]
    #[endpoint(unwrapEgld)]
    fn unwrap_egld(
        &self,
        #[payment] wrapped_egld_payment: BigUint,
        #[payment_token] token_identifier: TokenIdentifier,
    ) -> SCResult<SendEgld<BigUint>> {
        require!(
            !self.is_empty_wrapped_egld_token_identifier(),
            "Wrapped eGLD was not issued yet"
        );
        require!(token_identifier.is_esdt(), "Only ESDT tokens accepted");

        let wrapped_egld_token_identifier = self.get_wrapped_egld_token_identifier();

        require!(
            token_identifier == wrapped_egld_token_identifier,
            "Wrong esdt token"
        );
        require!(wrapped_egld_payment > 0, "Must pay more than 0 tokens!");
        // this should never happen, but we'll check anyway
        require!(
            wrapped_egld_payment <= self.get_sc_balance(),
            "Contract does not have enough funds"
        );

        self.add_total_wrapped(&wrapped_egld_token_identifier, &wrapped_egld_payment);

        Ok(SendEgld {
            to: self.get_caller(),
            amount: wrapped_egld_payment,
            data: BoxedBytes::from(&b"unwrapping"[..]),
        })
    }

    // views

    #[view(getLockedEgldBalance)]
    fn get_locked_egld_balance(&self) -> BigUint {
        self.get_sc_balance()
    }

    #[view(getTotalWrappedremaining)]
    fn get_total_wrapped_remaining(&self, token_identifier: &TokenIdentifier) -> BigUint {
        self.total_wrapped_remaining()
            .get(token_identifier)
            .unwrap_or_else(|| BigUint::zero())
    }

    #[view(wasTokenIssued)]
    fn was_token_issued(&self, token_identifier: &TokenIdentifier) -> bool {
        self.total_wrapped_remaining()
            .contains_key(token_identifier)
    }

    // private

    fn async_transfer_esdt(
        &self,
        to: Address,
        token_identifier: TokenIdentifier,
        amount: BigUint,
        func_name: BoxedBytes,
        args: &[BoxedBytes],
    ) -> AsyncCall<BigUint> {
        self.subtract_total_wrapped(&token_identifier, &amount);

        let mut contract_call_raw = ContractCall::new(to, token_identifier, amount, func_name);
        for arg in args {
            contract_call_raw.push_argument_raw_bytes(arg.as_slice());
        }

        contract_call_raw.async_call()
    }

    fn async_transfer_egld(
        &self,
        to: Address,
        amount: BigUint,
        func_name: BoxedBytes,
        args: &[BoxedBytes],
    ) -> AsyncCall<BigUint> {
        let mut contract_call_raw =
            ContractCall::new(to, TokenIdentifier::egld(), amount, func_name);
        for arg in args {
            contract_call_raw.push_argument_raw_bytes(arg.as_slice());
        }

        contract_call_raw.async_call()
    }

    fn complete_tx(
        &self,
        poly_tx_hash: &H256,
        tx_status: TransactionStatus,
    ) -> TransferEgldExecute<BigUint> {
        let mut contract_call_raw = ContractCall::new(
            self.get_cross_chain_management_contract_address(),
            TokenIdentifier::egld(),
            BigUint::zero(),
            BoxedBytes::from(COMPLETE_TX_ENDPOINT_NAME),
        );
        contract_call_raw.push_argument_raw_bytes(poly_tx_hash.as_bytes());
        contract_call_raw.push_argument_raw_bytes(&[tx_status as u8]);

        contract_call_raw.transfer_egld_execute()
    }

    fn add_total_wrapped(&self, token_identifier: &TokenIdentifier, amount: &BigUint) {
        let mut total_wrapped = self.get_total_wrapped_remaining(token_identifier);
        total_wrapped += amount;
        self.set_total_wrapped_remaining(token_identifier, &total_wrapped);
    }

    fn subtract_total_wrapped(&self, token_identifier: &TokenIdentifier, amount: &BigUint) {
        let mut total_wrapped = self.get_total_wrapped_remaining(token_identifier);
        total_wrapped -= amount;
        self.set_total_wrapped_remaining(token_identifier, &total_wrapped);
    }

    fn set_total_wrapped_remaining(&self, token_identifier: &TokenIdentifier, amount: &BigUint) {
        self.total_wrapped_remaining()
            .insert(token_identifier.clone(), amount.clone());
    }

    fn issue_esdt_token(
        &self,
        token_display_name: &BoxedBytes,
        token_ticker: &BoxedBytes,
        initial_supply: &BigUint,
        num_decimals: u8,
    ) -> AsyncCall<BigUint> {
        let mut contract_call_raw = ContractCall::new(
            Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY),
            TokenIdentifier::egld(),
            BigUint::from(ESDT_ISSUE_COST),
            BoxedBytes::from(ESDT_ISSUE_STRING),
        );

        contract_call_raw.push_argument_raw_bytes(token_display_name.as_slice());
        contract_call_raw.push_argument_raw_bytes(token_ticker.as_slice());
        contract_call_raw.push_argument_raw_bytes(&initial_supply.to_bytes_be());
        contract_call_raw.push_argument_raw_bytes(&[num_decimals]);

        contract_call_raw.push_argument_raw_bytes(&b"canFreeze"[..]);
        contract_call_raw.push_argument_raw_bytes(&b"false"[..]);

        contract_call_raw.push_argument_raw_bytes(&b"canWipe"[..]);
        contract_call_raw.push_argument_raw_bytes(&b"false"[..]);

        contract_call_raw.push_argument_raw_bytes(&b"canPause"[..]);
        contract_call_raw.push_argument_raw_bytes(&b"false"[..]);

        contract_call_raw.push_argument_raw_bytes(&b"canMint"[..]);
        contract_call_raw.push_argument_raw_bytes(&b"true"[..]);

        contract_call_raw.push_argument_raw_bytes(&b"canBurn"[..]);
        contract_call_raw.push_argument_raw_bytes(&b"true"[..]);

        contract_call_raw.push_argument_raw_bytes(&b"canChangeOwner"[..]);
        contract_call_raw.push_argument_raw_bytes(&b"false"[..]);

        contract_call_raw.push_argument_raw_bytes(&b"canUpgrade"[..]);
        contract_call_raw.push_argument_raw_bytes(&b"true"[..]);

        // save data for callback
        self.set_temporary_storage_esdt_operation(&self.get_tx_hash(), &EsdtOperation::Issue);

        contract_call_raw.async_call()
    }

    fn mint_esdt_token(
        &self,
        token_identifier: &TokenIdentifier,
        amount: &BigUint,
    ) -> AsyncCall<BigUint> {
        let mut contract_call_raw = ContractCall::new(
            Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY),
            TokenIdentifier::egld(),
            BigUint::zero(),
            BoxedBytes::from(ESDT_MINT_STRING),
        );
        contract_call_raw.push_argument_raw_bytes(token_identifier.as_slice());
        contract_call_raw.push_argument_raw_bytes(&amount.to_bytes_be());

        // save data for callback
        self.set_temporary_storage_esdt_operation(
            &self.get_tx_hash(),
            &EsdtOperation::Mint(token_identifier.clone(), amount.clone()),
        );

        contract_call_raw.async_call()
    }

    fn burn_esdt_token(
        &self,
        token_identifier: &TokenIdentifier,
        amount: &BigUint,
    ) -> AsyncCall<BigUint> {
        let mut contract_call_raw = ContractCall::new(
            Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY),
            TokenIdentifier::egld(),
            BigUint::zero(),
            BoxedBytes::from(ESDT_BURN_STRING),
        );
        contract_call_raw.push_argument_raw_bytes(token_identifier.as_slice());
        contract_call_raw.push_argument_raw_bytes(&amount.to_bytes_be());

        // save data for callback
        self.set_temporary_storage_esdt_operation(
            &self.get_tx_hash(),
            &EsdtOperation::Burn(token_identifier.clone(), amount.clone()),
        );

        contract_call_raw.async_call()
    }

    // callbacks

    #[callback_raw]
    fn callback_raw(
        &self,
        #[var_args] result: AsyncCallResult<VarArgs<BoxedBytes>>,
    ) -> OptionalResult<TransferEgldExecute<BigUint>> {
        let success = match result {
            AsyncCallResult::Ok(_) => true,
            AsyncCallResult::Err(_) => false,
        };
        let original_tx_hash = self.get_tx_hash();

        // if this is empty, it means this callBack comes from an issue ESDT call
        if self.is_empty_temporary_storage_poly_tx_hash(&original_tx_hash) {
            let esdt_operation = self.get_temporary_storage_esdt_operation(&original_tx_hash);
            match esdt_operation {
                // if this is also empty, then there is nothing to do in the callback
                EsdtOperation::None => {}
                EsdtOperation::Issue => self.perform_esdt_issue_callback(success),
                EsdtOperation::Mint(token_identifier, amount) => {
                    self.perform_esdt_mint_callback(success, &token_identifier, &amount)
                }
                EsdtOperation::Burn(token_identifier, amount) => {
                    self.perform_esdt_burn_callback(success, &token_identifier, &amount)
                }
            };

            self.clear_temporary_storage_esdt_operation(&original_tx_hash);

            OptionalResult::None
        } else {
            OptionalResult::Some(self.perform_async_callback(success, &original_tx_hash))
        }
    }

    fn perform_async_callback(
        &self,
        success: bool,
        original_tx_hash: &H256,
    ) -> TransferEgldExecute<BigUint> {
        let poly_tx_hash = self.get_temporary_storage_poly_tx_hash(&original_tx_hash);
        self.clear_temporary_storage_poly_tx_hash(&original_tx_hash);

        if success {
            self.complete_tx(&poly_tx_hash, TransactionStatus::Executed)
        } else {
            self.complete_tx(&poly_tx_hash, TransactionStatus::Rejected)
        }
    }

    fn perform_esdt_issue_callback(&self, success: bool) {
        // callback is called with ESDTTransfer of the newly issued token, with the amount requested,
        // so we can get the token identifier and initial supply from the call data
        let token_identifier = self.call_value().token();
        let initial_supply = self.call_value().esdt_value();

        if success {
            self.set_total_wrapped_remaining(&token_identifier, &initial_supply);
            self.set_last_issued_token_identifier(&token_identifier);

            // if this is empty, then this is the very first issue, which would be the wrapped eGLD token
            if self.is_empty_wrapped_egld_token_identifier() {
                self.set_wrapped_egld_token_identifier(&token_identifier);
            }
        }

        // nothing to do in case of error
    }

    fn perform_esdt_mint_callback(
        &self,
        success: bool,
        token_identifier: &TokenIdentifier,
        amount: &BigUint,
    ) {
        if success {
            self.add_total_wrapped(token_identifier, amount);
        }

        // nothing to do in case of error
    }

    fn perform_esdt_burn_callback(
        &self,
        success: bool,
        token_identifier: &TokenIdentifier,
        amount: &BigUint,
    ) {
        if success {
            self.subtract_total_wrapped(token_identifier, amount);
        }

        // nothing to do in case of error
    }

    // 1 eGLD = 1 wrapped eGLD, and they are interchangeable through this contract

    // TODO: Use improved SingleValueMapper in next elrond-wasm version if possible

    #[view(getWrappedEgldTokenIdentifier)]
    #[storage_get("wrappedEgldTokenIdentifier")]
    fn get_wrapped_egld_token_identifier(&self) -> TokenIdentifier;

    #[storage_set("wrappedEgldTokenIdentifier")]
    fn set_wrapped_egld_token_identifier(&self, token_identifier: &TokenIdentifier);

    #[storage_is_empty("wrappedEgldTokenIdentifier")]
    fn is_empty_wrapped_egld_token_identifier(&self) -> bool;

    // The total remaining wrapped tokens of each type owned by this SC.
    // Stored so we don't have to query everytime.

    #[storage_mapper("totalWrappedRemaining")]
    fn total_wrapped_remaining(&self) -> MapMapper<Self::Storage, TokenIdentifier, BigUint>;

    // Used to be able to issue, get the identifier, and then add it to whitelist in the other contracts

    #[view(getLastIssuedTokenIdentifier)]
    #[storage_get("lastIssuedTokenIdentifier")]
    fn get_last_issued_token_identifier(&self) -> TokenIdentifier;

    #[storage_set("lastIssuedTokenIdentifier")]
    fn set_last_issued_token_identifier(&self, token_identifier: &TokenIdentifier);

    // cross chain management

    #[storage_get("crossChainManagementContractAddress")]
    fn get_cross_chain_management_contract_address(&self) -> Address;

    #[storage_set("crossChainManagementContractAddress")]
    fn set_cross_chain_management_contract_address(&self, address: &Address);

    // ---------- Temporary storage for raw callbacks ----------

    // temporary storage for the poly_tx_hash, which is NOT the same as original_tx_hash
    // original_tx_hash is what you get when you call self.get_tx_hash() in the api
    // poly_tx_hash is the hash of the poly transaction

    #[storage_get("temporaryStoragePolyTxHash")]
    fn get_temporary_storage_poly_tx_hash(&self, original_tx_hash: &H256) -> H256;

    #[storage_set("temporaryStoragePolyTxHash")]
    fn set_temporary_storage_poly_tx_hash(&self, original_tx_hash: &H256, poly_tx_hash: &H256);

    #[storage_clear("temporaryStoragePolyTxHash")]
    fn clear_temporary_storage_poly_tx_hash(&self, original_tx_hash: &H256);

    #[storage_is_empty("temporaryStoragePolyTxHash")]
    fn is_empty_temporary_storage_poly_tx_hash(&self, original_tx_hash: &H256) -> bool;

    // temporary storage for ESDT operations. Used in callback.

    #[storage_get("temporaryStorageEsdtOperation")]
    fn get_temporary_storage_esdt_operation(
        &self,
        original_tx_hash: &H256,
    ) -> EsdtOperation<BigUint>;

    #[storage_set("temporaryStorageEsdtOperation")]
    fn set_temporary_storage_esdt_operation(
        &self,
        original_tx_hash: &H256,
        esdt_operation: &EsdtOperation<BigUint>,
    );

    #[storage_clear("temporaryStorageEsdtOperation")]
    fn clear_temporary_storage_esdt_operation(&self, original_tx_hash: &H256);

    #[storage_is_empty("temporaryStorageEsdtOperation")]
    fn is_empty_temporary_storage_esdt_operation(&self, original_tx_hash: &H256) -> bool;
}
