#![no_std]
#![allow(clippy::string_lit_as_bytes)]

use elrond_wasm::{derive_imports, imports, HexCallDataSerializer};

use transaction::TransactionStatus;

imports!();
derive_imports!();

// erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u
const ESDT_SYSTEM_SC_ADDRESS_ARRAY: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xff,
];

const ESDT_ISSUE_COST: u64 = 5000000000000000000; // 5 eGLD

const ESDT_TRANSFER_STRING: &[u8] = b"ESDTTransfer";
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
    ) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        require!(
            self.is_empty_wrapped_egld_token_identifier(),
            "wrapped egld was already issued"
        );

        require!(
            payment == BigUint::from(ESDT_ISSUE_COST),
            "Wrong payment, should pay exactly 5 eGLD for ESDT token issue"
        );

        self.issue_esdt_token(
            &BoxedBytes::from(WRAPPED_EGLD_DISPLAY_NAME),
            &BoxedBytes::from(WRAPPED_EGLD_TICKER),
            &initial_supply,
            EGLD_DECIMALS,
        );

        Ok(())
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
    ) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        require!(
            payment == BigUint::from(ESDT_ISSUE_COST),
            "Wrong payment, should pay exactly 5 eGLD for ESDT token issue"
        );

        require!(initial_supply > 0, "initial supply must be more than 0");

        self.issue_esdt_token(
            &token_display_name,
            &token_ticker,
            &initial_supply,
            num_decimals,
        );

        Ok(())
    }

    #[endpoint(mintEsdtToken)]
    fn mint_esdt_token_endpoint(
        &self,
        token_identifier: TokenIdentifier,
        amount: BigUint,
    ) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        require!(
            self.was_token_issued(&token_identifier),
            "Token must be issued first"
        );
        require!(amount > 0, "Amount minted must be more than 0");

        self.mint_esdt_token(&token_identifier, &amount);

        Ok(())
    }

    #[endpoint(burnEsdtToken)]
    fn burn_esdt_token_endpoint(
        &self,
        token_identifier: TokenIdentifier,
        amount: BigUint,
    ) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        require!(amount > 0, "Amount burned must be more than 0");
        require!(
            amount < self.get_total_wrapped_remaining(&token_identifier),
            "Can't burn more than total wrapped remaining"
        );

        self.burn_esdt_token(&token_identifier, &amount);

        Ok(())
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
    ) -> SCResult<()> {
        require!(
            self.get_caller() == self.get_cross_chain_management_contract_address(),
            "Only the cross chain management contract may call this function"
        );

        let total_wrapped = self.get_total_wrapped_remaining(&token_identifier);
        if total_wrapped < amount {
            self.complete_tx(&poly_tx_hash, TransactionStatus::OutOfFunds);
        } else {
            // save the poly_tx_hash to be used in the callback
            self.set_temporary_storage_poly_tx_hash(&self.get_tx_hash(), &poly_tx_hash);

            if token_identifier != self.get_wrapped_egld_token_identifier() {
                self.transfer_esdt(&token_identifier, &amount, &to, &func_name, args.as_slice());
            } else {
                // automatically unwrap before sending if the token is wrapped eGLD
                self.transfer_egld(&to, &amount, &func_name, args.as_slice());

                // TODO: Update contract wrapped eGLD balance in storage
            }
        }

        Ok(())
    }

    // endpoints

    #[payable("EGLD")]
    #[endpoint(wrapEgld)]
    fn wrap_egld(&self, #[payment] payment: BigUint) -> SCResult<()> {
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

        self.transfer_esdt(
            &wrapped_egld_token_identifier,
            &payment,
            &self.get_caller(),
            &BoxedBytes::empty(),
            &[],
        );

        Ok(())
    }

    #[payable("*")]
    #[endpoint(unwrapEgld)]
    fn unwrap_egld(
        &self,
        #[payment] wrapped_egld_payment: BigUint,
        #[payment_token] token_identifier: TokenIdentifier,
    ) -> SCResult<()> {
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

        // 1 wrapped eGLD = 1 eGLD, so we pay back the same amount
        self.send()
            .direct_egld(&self.get_caller(), &wrapped_egld_payment, b"unwrapping");

        Ok(())
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

    ///////////////////////////////////////// TODO ////////////////////////////////////////////////////
    // 1) Rework the flow for simple transfers. Only rely on async_call_raw and callbacks for actual scCalls
    // 2) Don't use send().direct_egl() for scCalls, as it doesn't work

    fn transfer_esdt(
        &self,
        token_identifier: &TokenIdentifier,
        amount: &BigUint,
        to: &Address,
        func_name: &BoxedBytes,
        args: &[BoxedBytes],
    ) {
        let mut serializer = HexCallDataSerializer::new(ESDT_TRANSFER_STRING);
        serializer.push_argument_bytes(token_identifier.as_slice());
        serializer.push_argument_bytes(amount.to_bytes_be().as_slice());

        if !func_name.is_empty() {
            serializer.push_argument_bytes(func_name.as_slice());
            for arg in args {
                serializer.push_argument_bytes(arg.as_slice());
            }
        }

        self.subtract_total_wrapped(token_identifier, amount);

        self.send()
            .async_call_raw(to, &BigUint::zero(), serializer.as_slice());
    }

    fn transfer_egld(
        &self,
        to: &Address,
        amount: &BigUint,
        func_name: &BoxedBytes,
        args: &[BoxedBytes],
    ) {
        if !func_name.is_empty() {
            let mut serializer = HexCallDataSerializer::new(func_name.as_slice());

            for arg in args {
                serializer.push_argument_bytes(arg.as_slice());
            }

            self.send()
                .async_call_raw(to, amount, serializer.as_slice());
        } else {
            self.send().async_call_raw(to, amount, &[]);
        }
    }

    fn complete_tx(&self, poly_tx_hash: &H256, tx_status: TransactionStatus) {
        let mut serializer = HexCallDataSerializer::new(COMPLETE_TX_ENDPOINT_NAME);
        serializer.push_argument_bytes(poly_tx_hash.as_bytes());
        serializer.push_argument_bytes(&[tx_status as u8]);

        // set status in the cross chain management contract
        self.send().direct_egld(
            &self.get_cross_chain_management_contract_address(),
            &BigUint::zero(),
            serializer.as_slice(),
        );
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
    ) {
        let mut serializer = HexCallDataSerializer::new(ESDT_ISSUE_STRING);

        serializer.push_argument_bytes(token_display_name.as_slice());
        serializer.push_argument_bytes(token_ticker.as_slice());
        serializer.push_argument_bytes(&initial_supply.to_bytes_be());
        serializer.push_argument_bytes(&[num_decimals]);

        serializer.push_argument_bytes(&b"canFreeze"[..]);
        serializer.push_argument_bytes(&b"false"[..]);

        serializer.push_argument_bytes(&b"canWipe"[..]);
        serializer.push_argument_bytes(&b"false"[..]);

        serializer.push_argument_bytes(&b"canPause"[..]);
        serializer.push_argument_bytes(&b"false"[..]);

        serializer.push_argument_bytes(&b"canMint"[..]);
        serializer.push_argument_bytes(&b"true"[..]);

        serializer.push_argument_bytes(&b"canBurn"[..]);
        serializer.push_argument_bytes(&b"true"[..]);

        serializer.push_argument_bytes(&b"canChangeOwner"[..]);
        serializer.push_argument_bytes(&b"false"[..]);

        serializer.push_argument_bytes(&b"canUpgrade"[..]);
        serializer.push_argument_bytes(&b"false"[..]);

        // save data for callback
        self.set_temporary_storage_esdt_operation(&self.get_tx_hash(), &EsdtOperation::Issue);

        self.send().async_call_raw(
            &Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY),
            &BigUint::from(ESDT_ISSUE_COST),
            serializer.as_slice(),
        );
    }

    fn mint_esdt_token(&self, token_identifier: &TokenIdentifier, amount: &BigUint) {
        let mut serializer = HexCallDataSerializer::new(ESDT_MINT_STRING);
        serializer.push_argument_bytes(token_identifier.as_slice());
        serializer.push_argument_bytes(&amount.to_bytes_be());

        // save data for callback
        self.set_temporary_storage_esdt_operation(
            &self.get_tx_hash(),
            &EsdtOperation::Mint(token_identifier.clone(), amount.clone()),
        );

        self.send().async_call_raw(
            &Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY),
            &BigUint::zero(),
            serializer.as_slice(),
        );
    }

    fn burn_esdt_token(&self, token_identifier: &TokenIdentifier, amount: &BigUint) {
        let mut serializer = HexCallDataSerializer::new(ESDT_BURN_STRING);
        serializer.push_argument_bytes(token_identifier.as_slice());
        serializer.push_argument_bytes(&amount.to_bytes_be());

        // save data for callback
        self.set_temporary_storage_esdt_operation(
            &self.get_tx_hash(),
            &EsdtOperation::Burn(token_identifier.clone(), amount.clone()),
        );

        self.send().async_call_raw(
            &Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY),
            &BigUint::zero(),
            serializer.as_slice(),
        );
    }

    // callbacks

    #[callback_raw]
    fn callback_raw(&self, #[var_args] result: AsyncCallResult<VarArgs<BoxedBytes>>) {
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
                EsdtOperation::None => return,
                EsdtOperation::Issue => self.perform_esdt_issue_callback(success),
                EsdtOperation::Mint(token_identifier, amount) => {
                    self.perform_esdt_mint_callback(success, &token_identifier, &amount)
                }
                EsdtOperation::Burn(token_identifier, amount) => {
                    self.perform_esdt_burn_callback(success, &token_identifier, &amount)
                }
            };

            self.clear_temporary_storage_esdt_operation(&original_tx_hash);
        } else {
            self.perform_async_callback(success, &original_tx_hash);

            self.clear_temporary_storage_poly_tx_hash(&original_tx_hash);
        }
    }

    fn perform_async_callback(&self, success: bool, original_tx_hash: &H256) {
        let poly_tx_hash = self.get_temporary_storage_poly_tx_hash(&original_tx_hash);

        if success {
            self.complete_tx(&poly_tx_hash, TransactionStatus::Executed);
        } else {
            self.complete_tx(&poly_tx_hash, TransactionStatus::Rejected);
        }
    }

    fn perform_esdt_issue_callback(&self, success: bool) {
        // callback is called with ESDTTransfer of the newly issued token, with the amount requested, so we can get the token identifier from the call data
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

    // STORAGE TODO: Use storage mappers

    // 1 eGLD = 1 wrapped eGLD, and they are interchangeable through this contract

    #[view(getWrappedEgldTokenIdentifier)]
    #[storage_get("wrappedEgldTokenIdentifier")]
    fn get_wrapped_egld_token_identifier(&self) -> TokenIdentifier;

    #[storage_set("wrappedEgldTokenIdentifier")]
    fn set_wrapped_egld_token_identifier(&self, token_identifier: &TokenIdentifier);

    #[storage_is_empty("wrappedEgldTokenIdentifier")]
    fn is_empty_wrapped_egld_token_identifier(&self) -> bool;

    // The total remaining wrapped tokens of each type owned by this SC. Stored so we don't have to query everytime.

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
