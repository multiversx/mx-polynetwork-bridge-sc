#![no_std]
#![allow(clippy::string_lit_as_bytes)]

use elrond_wasm::{HexCallDataSerializer, only_owner};

imports!();

const ESDT_TRANSFER_STRING: &[u8] = b"ESDTTransfer";

#[elrond_wasm_derive::contract(SimpleEsdtImpl)]
pub trait SimpleEsdt {

	#[init]
	fn init(&self, cross_chain_management_address: Address, wrapped_egld_token_name: BoxedBytes) {
		self.set_cross_chain_management_contract_address(&cross_chain_management_address);
		self.set_wrapped_egld_token_name(&wrapped_egld_token_name);
	}

	// endpoints - owner-only

	#[endpoint(supplyTokens)]
	fn supply_tokens(&self) -> SCResult<()> {
		only_owner!(self, "Only owner may call this function");

		let token_name = self.get_esdt_token_name_boxed();
		let wrapped_token_payment = self.get_esdt_value_big_uint();

		let mut total_wrapped = self.get_total_wrapped_remaining(&token_name);
		total_wrapped += wrapped_token_payment;
		self.set_total_wrapped_remaining(&token_name, &total_wrapped);

		Ok(())
	}

	// endpoints

	#[payable]
	#[endpoint(wrapEgld)]
	fn wrap_egld(&self, #[payment] payment: BigUint) -> SCResult<()> {
		require!(payment > 0, "Payment must be more than 0");

		let wrapped_egld_token_name = self.get_wrapped_egld_token_name();
		let wrapped_egld_left = self.get_total_wrapped_remaining(&wrapped_egld_token_name);

		require!(wrapped_egld_left >= payment, 
			"Contract does not have enough wrapped eGLD. Please try again once more is minted.");

		self.transfer_esdt_to_account(&wrapped_egld_token_name, &payment, &self.get_caller());

		Ok(())
	}

	#[endpoint(unwrapEgld)]
	fn unwrap_egld(&self) -> SCResult<()> {
		let esdt_token_name = self.get_esdt_token_name_boxed();
		let wrapped_egld_token_name = self.get_wrapped_egld_token_name();

		require!(esdt_token_name == wrapped_egld_token_name, "Wrong esdt token");

		let wrapped_egld_payment = self.get_esdt_value_big_uint();

		require!(wrapped_egld_payment > 0, "Must pay more than 0 tokens!");
		// this should never happen, but we'll check anyway
		require!(wrapped_egld_payment <= self.get_sc_balance(), "Contract does not have enough funds");

		let mut wrapped_egld_remaining = self.get_total_wrapped_remaining(&wrapped_egld_token_name);
		wrapped_egld_remaining += &wrapped_egld_payment;
		self.set_total_wrapped_remaining(&wrapped_egld_token_name, &wrapped_egld_remaining);

		// 1 wrapped eGLD = 1 eGLD, so we pay back the same amount
		self.send_tx(&self.get_caller(), &wrapped_egld_payment, b"unwrapping");

		Ok(())
	}

	// private

	fn get_esdt_token_name_boxed(&self) -> BoxedBytes {
		BoxedBytes::from(self.get_esdt_token_name())
	}

	fn transfer_esdt_to_account(&self, esdt_token_name: &BoxedBytes, amount: &BigUint, to: &Address) {
		let mut serializer = HexCallDataSerializer::new(ESDT_TRANSFER_STRING);
		serializer.push_argument_bytes(esdt_token_name.as_slice());
		serializer.push_argument_bytes(amount.to_bytes_be().as_slice());

		self.send_tx(&to, &BigUint::zero(), serializer.as_slice());
	}

	fn transfer_esdt_to_contract(&self, esdt_token_name: &BoxedBytes, amount: &BigUint, to: &Address, 
		func_name: BoxedBytes, args: Vec<BoxedBytes>) {

		let mut serializer = HexCallDataSerializer::new(ESDT_TRANSFER_STRING);
		serializer.push_argument_bytes(esdt_token_name.as_slice());
		serializer.push_argument_bytes(amount.to_bytes_be().as_slice());

		serializer.push_argument_bytes(func_name.as_slice());
		for arg in &args {
			serializer.push_argument_bytes(arg.as_slice());
		}

		self.async_call(&to, &BigUint::zero(), serializer.as_slice());
	}

	// STORAGE

	// 1 eGLD = 1 wrapped eGLD, and they are interchangeable through this contract

	#[view(getWrappedEgldTokenName)]
	#[storage_get("wrappedEgldTokenName")]
	fn get_wrapped_egld_token_name(&self) -> BoxedBytes;

	#[storage_set("wrappedEgldTokenName")]
	fn set_wrapped_egld_token_name(&self, token_name: &BoxedBytes);

	// Each chain will have its own token. New types of tokens will be issues/minted as needed by this contract

	#[view(getTokenNameForChain)]
	#[storage_get("tokenNameForChain")]
	fn get_token_name_for_chain(&self, chain_id: u64) -> BoxedBytes;

	#[storage_set("tokenNameForChain")]
	fn set_token_name_for_chain(&self, chain_id: u64, token_name: &BoxedBytes);

	#[storage_is_empty("tokenNameForChain")]
	fn is_empty_token_name_for_chain(&self, chain_id: u64) -> bool;

	// The total remaining wrapped tokens of each type owned by this SC. Stored so we don't have to query everytime.

	#[view(getTotalWrapped)]
	#[storage_get("totalWrappedRemaining")]
	fn get_total_wrapped_remaining(&self, token_wrapped: &BoxedBytes) -> BigUint;

	#[storage_set("totalWrappedRemaining")]
	fn set_total_wrapped_remaining(&self, token_name: &BoxedBytes , total_wrapped: &BigUint);
	
	// ---

	#[storage_get("crossChainManagementContractAddress")]
	fn get_cross_chain_management_contract_address(&self) -> Address;

	#[storage_set("crossChainManagementContractAddress")]
	fn set_cross_chain_management_contract_address(&self, address: &Address);
}
