#![no_std]
#![allow(clippy::string_lit_as_bytes)]
#![allow(non_snake_case)]

use transaction::TransactionStatus;

elrond_wasm::imports!();

pub mod transfer_esdt_action_result;
use transfer_esdt_action_result::*;

const WRAPPED_EGLD_DISPLAY_NAME: &[u8] = b"WrappedEGLD";
const WRAPPED_EGLD_TICKER: &[u8] = b"WEGLD";
const EGLD_DECIMALS: usize = 18;

#[elrond_wasm_derive::callable(CrossChainManagementProxy)]
pub trait CrossChainManagement {
    fn completeTx(
        &self,
        poly_tx_hash: H256,
        tx_status: TransactionStatus,
    ) -> ContractCall<BigUint, ()>;
}

#[elrond_wasm_derive::contract(EsdtTokenManagerImpl)]
pub trait EsdtTokenManager {
    #[init]
    fn init(&self, cross_chain_management_address: Address) {
        self.cross_chain_management_contract_address()
            .set(&cross_chain_management_address);
    }

    // endpoints - owner-only

    #[payable("EGLD")]
    #[endpoint(performWrappedEgldIssue)]
    fn perform_wrapped_egld_issue(
        &self,
        initial_supply: BigUint,
        #[payment] issue_cost: BigUint,
    ) -> SCResult<AsyncCall<BigUint>> {
        only_owner!(self, "only owner may call this function");

        require!(
            self.wrapped_egld_token_identifier().is_empty(),
            "wrapped egld was already issued"
        );

        Ok(ESDTSystemSmartContractProxy::new()
            .issue_fungible(
                issue_cost,
                &BoxedBytes::from(WRAPPED_EGLD_DISPLAY_NAME),
                &BoxedBytes::from(WRAPPED_EGLD_TICKER),
                &initial_supply,
                FungibleTokenProperties {
                    num_decimals: EGLD_DECIMALS,
                    can_freeze: false,
                    can_wipe: false,
                    can_pause: false,
                    can_mint: true,
                    can_burn: true,
                    can_change_owner: false,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(self.callbacks().esdt_issue_callback()))
    }

    #[payable("EGLD")]
    #[endpoint(issueEsdtToken)]
    fn issue_esdt_token_endpoint(
        &self,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
        initial_supply: BigUint,
        num_decimals: usize,
        #[payment] issue_cost: BigUint,
    ) -> SCResult<AsyncCall<BigUint>> {
        only_owner!(self, "only owner may call this function");

        require!(initial_supply > 0, "initial supply must be more than 0");

        Ok(ESDTSystemSmartContractProxy::new()
            .issue_fungible(
                issue_cost,
                &token_display_name,
                &token_ticker,
                &initial_supply,
                FungibleTokenProperties {
                    num_decimals,
                    can_freeze: false,
                    can_wipe: false,
                    can_pause: false,
                    can_mint: true,
                    can_burn: true,
                    can_change_owner: false,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(self.callbacks().esdt_issue_callback()))
    }

    #[endpoint(setLocalMintRole)]
    fn set_local_mint_role(&self, token_id: TokenIdentifier) -> SCResult<AsyncCall<BigUint>> {
        only_owner!(self, "only owner may call this function");

        require!(
            self.issued_tokens().contains(&token_id),
            "Token was not issued yet"
        );
        require!(
            !self.are_local_roles_set(&token_id).get(),
            "Local roles were already set"
        );

        Ok(ESDTSystemSmartContractProxy::new()
            .set_special_roles(
                &self.get_sc_address(),
                token_id.as_esdt_identifier(),
                &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
            )
            .async_call()
            .with_callback(self.callbacks().set_roles_callback(token_id)))
    }

    #[endpoint(mintEsdtToken)]
    fn mint_esdt_token(&self, token_id: TokenIdentifier, amount: BigUint) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        require!(
            self.was_token_issued(&token_id),
            "Token must be issued first"
        );
        require!(amount > 0, "Amount minted must be more than 0");

        self.try_mint(&token_id, &amount)
    }

    // endpoints - CrossChainManagement contract - only

    #[endpoint(transferEsdt)]
    fn transfer_esdt(
        &self,
        token_id: TokenIdentifier,
        amount: BigUint,
        to: Address,
        poly_tx_hash: H256,
        func_name: BoxedBytes,
        #[var_args] args: VarArgs<BoxedBytes>,
    ) -> SCResult<TransferEsdtActionResult<BigUint>> {
        require!(
            self.get_caller() == self.cross_chain_management_contract_address().get(),
            "Only the cross chain management contract may call this function"
        );
        require!(
            self.was_token_issued(&token_id),
            "Token must be issued first"
        );

        let total_wrapped = self.get_total_wrapped_remaining(&token_id);
        if total_wrapped < amount {
            let extra_needed = &amount - &total_wrapped;

            sc_try!(self.try_mint(&token_id, &extra_needed));
        }

        if self.is_smart_contract(&to) {
            if token_id != self.wrapped_egld_token_identifier().get() {
                Ok(TransferEsdtActionResult::AsyncCall(
                    self.async_transfer_esdt(to, token_id, amount, func_name, args.as_slice())
                        .with_callback(self.callbacks().async_transfer_callback(poly_tx_hash)),
                ))
            } else {
                // automatically unwrap before sending if the token is wrapped eGLD
                Ok(TransferEsdtActionResult::AsyncCall(
                    self.async_transfer_egld(to, amount, func_name, args.as_slice()),
                ))
            }
        } else {
            self.send().direct_esdt_via_transf_exec(
                &to,
                token_id.as_esdt_identifier(),
                &amount,
                b"offchain transfer",
            );

            Ok(TransferEsdtActionResult::TransferEgldExecute(
                self.complete_tx(poly_tx_hash, TransactionStatus::Executed),
            ))
        }
    }

    // endpoints

    #[payable("EGLD")]
    #[endpoint(wrapEgld)]
    fn wrap_egld(&self, #[payment] payment: BigUint) -> SCResult<SendEsdt<BigUint>> {
        require!(payment > 0, "Payment must be more than 0");
        require!(
            !self.wrapped_egld_token_identifier().is_empty(),
            "Wrapped eGLD was not issued yet"
        );

        let wrapped_egld_token_identifier = self.wrapped_egld_token_identifier().get();
        let wrapped_egld_left = self.get_total_wrapped_remaining(&wrapped_egld_token_identifier);

        require!(
            wrapped_egld_left >= payment,
            "Contract does not have enough wrapped eGLD. Please try again once more is minted."
        );

        let caller = self.get_caller();
        let data = BoxedBytes::from(self.data_or_empty(&caller, b"wrapping"));
        Ok(SendEsdt {
            to: caller,
            token_name: wrapped_egld_token_identifier.into_boxed_bytes(),
            amount: payment,
            data,
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
            !self.wrapped_egld_token_identifier().is_empty(),
            "Wrapped eGLD was not issued yet"
        );
        require!(token_identifier.is_esdt(), "Only ESDT tokens accepted");

        let wrapped_egld_token_identifier = self.wrapped_egld_token_identifier().get();

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

        let caller = self.get_caller();
        let data = BoxedBytes::from(self.data_or_empty(&caller, b"unwrapping"));
        Ok(SendEgld {
            to: caller,
            amount: wrapped_egld_payment,
            data,
        })
    }

    // views

    #[view(getLockedEgldBalance)]
    fn get_locked_egld_balance(&self) -> BigUint {
        self.get_sc_balance()
    }

    #[view(getTotalWrappedremaining)]
    fn get_total_wrapped_remaining(&self, token_id: &TokenIdentifier) -> BigUint {
        self.get_esdt_balance(&self.get_sc_address(), token_id.as_esdt_identifier(), 0)
    }

    #[view(wasTokenIssued)]
    fn was_token_issued(&self, token_id: &TokenIdentifier) -> bool {
        self.issued_tokens().contains(token_id)
    }

    // private

    fn async_transfer_esdt(
        &self,
        to: Address,
        token_id: TokenIdentifier,
        amount: BigUint,
        func_name: BoxedBytes,
        args: &[BoxedBytes],
    ) -> AsyncCall<BigUint> {
        let mut contract_call_raw =
            ContractCall::<BigUint, ()>::new(to, token_id, amount, func_name);
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
            ContractCall::<BigUint, ()>::new(to, TokenIdentifier::egld(), amount, func_name);
        for arg in args {
            contract_call_raw.push_argument_raw_bytes(arg.as_slice());
        }

        contract_call_raw.async_call()
    }

    fn complete_tx(
        &self,
        poly_tx_hash: H256,
        tx_status: TransactionStatus,
    ) -> TransferEgldExecute<BigUint> {
        contract_call!(
            self,
            self.cross_chain_management_contract_address().get(),
            CrossChainManagementProxy
        )
        .completeTx(poly_tx_hash, tx_status)
        .transfer_egld_execute()
    }

    fn try_mint(&self, token_id: &TokenIdentifier, amount: &BigUint) -> SCResult<()> {
        require!(
            self.are_local_roles_set(token_id).get(),
            "LocalMint role not set"
        );

        self.send()
            .esdt_local_mint(self.get_gas_left(), token_id.as_esdt_identifier(), &amount);

        Ok(())
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
    fn esdt_issue_callback(
        &self,
        #[payment_token] token_identifier: TokenIdentifier,
        #[payment] returned_tokens: BigUint,
        #[call_result] result: AsyncCallResult<()>,
    ) {
        // callback is called with ESDTTransfer of the newly issued token, with the amount requested,
        // so we can get the token identifier and amount from the call data
        match result {
            AsyncCallResult::Ok(()) => {
                // if this is empty, then this is the very first issue, which would be the wrapped eGLD token
                if self.wrapped_egld_token_identifier().is_empty() {
                    self.wrapped_egld_token_identifier().set(&token_identifier);
                }

                self.last_issued_token_identifier().set(&token_identifier);
                self.issued_tokens().insert(token_identifier);
            }
            AsyncCallResult::Err(_) => {
                // refund payment to caller, which is the sc owner
                if token_identifier.is_egld() && returned_tokens > 0 {
                    self.send()
                        .direct_egld(&self.get_owner_address(), &returned_tokens, &[]);
                }
            }
        }
    }

    #[callback]
    fn set_roles_callback(
        &self,
        token_id: TokenIdentifier,
        #[call_result] result: AsyncCallResult<()>,
    ) {
        match result {
            AsyncCallResult::Ok(()) => {
                self.are_local_roles_set(&token_id).set(&true);
            }
            AsyncCallResult::Err(_) => {}
        }
    }

    #[callback]
    fn async_transfer_callback(
        &self,
        poly_tx_hash: H256,
        #[call_result] result: AsyncCallResult<()>,
    ) -> TransferEgldExecute<BigUint> {
        match result {
            AsyncCallResult::Ok(()) => self.complete_tx(poly_tx_hash, TransactionStatus::Executed),
            AsyncCallResult::Err(_) => self.complete_tx(poly_tx_hash, TransactionStatus::Rejected),
        }
    }

    // 1 eGLD = 1 wrapped eGLD, and they are interchangeable through this contract

    #[view(getWrappedEgldTokenIdentifier)]
    #[storage_mapper("wrappedEgldTokenIdentifier")]
    fn wrapped_egld_token_identifier(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    // Used to be able to issue, get the identifier, and then add it to whitelist in the other contracts

    #[view(getLastIssuedTokenIdentifier)]
    #[storage_mapper("lastIssuedTokenIdentifier")]
    fn last_issued_token_identifier(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    // cross chain management

    #[storage_mapper("crossChainManagementContractAddress")]
    fn cross_chain_management_contract_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(areLocalRolesSet)]
    #[storage_mapper("areLocalRolesSet")]
    fn are_local_roles_set(
        &self,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<Self::Storage, bool>;

    #[storage_mapper("issuedTokens")]
    fn issued_tokens(&self) -> SetMapper<Self::Storage, TokenIdentifier>;
}
