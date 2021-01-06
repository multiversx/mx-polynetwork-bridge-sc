#![no_std]
#![allow(clippy::string_lit_as_bytes)]

use elrond_wasm::{HexCallDataSerializer, only_owner};

imports!();

const ESDT_SYSTEM_SC_ADDRESS_ARRAY: [u8;32] = [ 
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
	0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
	0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xff];

#[elrond_wasm_derive::contract(SimpleEsdtImpl)]
pub trait SimpleEsdt {
	// STORAGE

	/// Tokens left
	#[view(totalSupply)]
	#[storage_get("tokens_left")]
	fn get_tokens_left(&self) -> BigUint;

	#[storage_set("tokens_left")]
	fn set_tokens_left(&self, tokens_left: &BigUint);

	#[storage_get("token_name")]
	fn get_token_name(&self) -> BoxedBytes;

	#[storage_set("token_name")]
	fn set_token_name(&self, token_name: &BoxedBytes);

	#[storage_get("crossChainManagementContractAddress")]
	fn get_cross_chain_management_contract_address(&self) -> Address;

	#[storage_set("crossChainManagementContractAddress")]
	fn set_cross_chain_management_contract_address(&self, address: &Address);

	// FUNCTIONALITY

	/// Constructor, is called immediately after the contract is created
	#[init]
	fn init(&self, cross_chain_management_address: Address) {
		self.set_cross_chain_management_contract_address(&cross_chain_management_address);
	}

	// Should be called after init
	// This can't be done in the init function, as the contract doesn't have an address yet
	#[endpoint(mintInitialTokens)]
	fn mint_initial_tokens(&self, token_name: BoxedBytes, token_ticker: BoxedBytes,
		total_supply: BigUint) -> SCResult<()> {

		only_owner!(self, "only owner may call this function!");
		require!(self.get_token_name() == BoxedBytes::empty(), "Initial tokens already created!");

		self.set_token_name(&token_name);
		self.set_tokens_left(&total_supply);

		self.issue_tokens(&token_name, &token_ticker, &total_supply);

		Ok(())
	}

	/// Transfer token to a specified address from sender.
	#[endpoint]
	fn transfer(&self, to: Address, amount: BigUint) -> SCResult<()> {
		require!(self.get_caller() == self.get_cross_chain_management_contract_address(), 
			"Only the Cross Chain Management SC may call this function");

		let mut tokens_left = self.get_tokens_left();

		require!(amount < tokens_left, "Not enough tokens remaning, mint more!");

		tokens_left -= &amount;
		self.set_tokens_left(&tokens_left);

		let mut hex = HexCallDataSerializer::new(&b"ESDTTransfer"[..]);
		hex.push_argument_bytes(self.get_token_name().as_slice());
		hex.push_argument_bytes(&amount.to_bytes_be());

		self.async_call(&to, &BigUint::zero(), hex.as_slice());

		Ok(())
	}

	// mint more tokens
	#[endpoint]
	fn mint(&self, amount: &BigUint) -> SCResult<()> {
		require!(self.get_caller() == self.get_owner_address(), 
			"Only the owner may call this function");

		let mut tokens_left = self.get_tokens_left();
		tokens_left += amount;
		self.set_tokens_left(&tokens_left);

		let mut hex = HexCallDataSerializer::new(&b"mint"[..]);
		hex.push_argument_bytes(self.get_token_name().as_slice());
		hex.push_argument_bytes(&amount.to_bytes_be());

		self.async_call(&Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY), &BigUint::zero(), hex.as_slice());

		Ok(())
	}

	// issue the initial tokens
	fn issue_tokens(&self, token_name: &BoxedBytes, token_ticker: &BoxedBytes, 
		total_supply: &BigUint) {

		let mut hex = HexCallDataSerializer::new(&b"issue"[..]);
		hex.push_argument_bytes(token_name.as_slice());
		hex.push_argument_bytes(token_ticker.as_slice());
		hex.push_argument_bytes(&total_supply.to_bytes_be());

		hex.push_argument_bytes(&b"canFreeze"[..]);
		hex.push_argument_bytes(&b"false"[..]);

		hex.push_argument_bytes(&b"canWipe"[..]);
		hex.push_argument_bytes(&b"false"[..]);

		hex.push_argument_bytes(&b"canPause"[..]);
		hex.push_argument_bytes(&b"false"[..]);

		hex.push_argument_bytes(&b"canMint"[..]);
		hex.push_argument_bytes(&b"true"[..]);

		hex.push_argument_bytes(&b"canBurn"[..]);
		hex.push_argument_bytes(&b"false"[..]);

		hex.push_argument_bytes(&b"canChangeOwner"[..]);
		hex.push_argument_bytes(&b"false"[..]);

		hex.push_argument_bytes(&b"canUpgrade"[..]);
		hex.push_argument_bytes(&b"false"[..]);

		self.async_call(&Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY), &BigUint::zero(), hex.as_slice());
	}
}
