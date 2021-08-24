#![no_std]

use transaction::TransactionArgs;

elrond_wasm::imports!();

pub const UNLOCK_METHOD_NAME: &[u8] = b"unlock";

mod cross_chain_management_proxy {
    use transaction::TransactionArgs;

    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait CrossChainManagement {
        #[payable("*")]
        #[endpoint(createCrossChainTx)]
        fn create_cross_chain_tx(
            &self,
            to_chain_id: u64,
            to_contract_address: BoxedBytes,
            method_name: BoxedBytes,
            method_args: TransactionArgs<Self::BigUint>,
        );
    }
}

#[elrond_wasm::contract]
pub trait TransactionRelayer {
    #[init]
    fn init(&self) {}

    // endpoints - owner-only

    #[only_owner]
    #[endpoint]
    fn unlock(
        &self,
        args: TransactionArgs<Self::BigUint>,
        from_contract_address: BoxedBytes,
        from_chain_id: u64,
    ) -> SCResult<AsyncCall<Self::SendApi>> {
        require!(
            !from_contract_address.is_empty(),
            "from_contract_address cannot be empty"
        );

        let from_proxy_contract = self.proxy_hash_map(from_chain_id).get();
        require!(
            from_contract_address == from_proxy_contract,
            "from_contract_address is not the expected proxy contract address"
        );

        require!(!args.asset_hash.is_empty(), "asset_hash cannot be empty");

        let elrond_dest_address = self.try_convert_to_elrond_address(&args.dest_address)?;
        require!(
            !self.blockchain().is_smart_contract(&elrond_dest_address),
            "cannot transfer to smart contract"
        );

        let token_id = TokenIdentifier::from(args.asset_hash.as_slice());
        require!(
            token_id.is_valid_esdt_identifier(),
            "Invalid Token ID provided"
        );
        self.try_mint(&token_id, &args.amount)?;

        self.unlock_event(&token_id, &elrond_dest_address, &args.amount);

        Ok(self.async_transfer_esdt(elrond_dest_address, token_id, args.amount))
    }

    // endpoints

    #[payable("*")]
    #[endpoint]
    fn lock(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_amount] payment_amount: Self::BigUint,
        to_chain_id: u64,
        dest_address: BoxedBytes,
    ) -> SCResult<()> {
        require!(
            payment_token.is_esdt(),
            "eGLD payment not allowed, wrap your EGLD first"
        );
        require!(
            self.call_value().esdt_token_nonce() == 0,
            "Can only bridge fungible ESDTs"
        );
        require!(payment_amount > 0, "Must bridge more than 0 tokens");

        let dest_chain_proxy = self.proxy_hash_map(to_chain_id).get();
        require!(
            !dest_chain_proxy.is_empty(),
            "Selected Chain ID not supported"
        );

        let to_asset_hash = self.asset_hash_map(&payment_token, to_chain_id).get();
        require!(
            !to_asset_hash.is_empty(),
            "This specific token cannot be bridged"
        );

        self.try_burn(&payment_token, &payment_amount)?;

        let caller = self.blockchain().get_caller();
        let tx_args = TransactionArgs {
            dest_address,
            asset_hash: to_asset_hash,
            amount: payment_amount,
        };

        let ccm_address = self.get_cross_chain_management_sc_address();
        self.cross_chain_management_proxy(ccm_address)
            .create_cross_chain_tx(
                to_chain_id,
                dest_chain_proxy,
                UNLOCK_METHOD_NAME.into(),
                tx_args.clone(),
            )
            .execute_on_dest_context();

        self.lock_event(
            &payment_token,
            &caller,
            to_chain_id,
            &tx_args.asset_hash,
            &tx_args.dest_address,
            &tx_args.amount,
        );

        Ok(())
    }

    // private

    fn get_cross_chain_management_sc_address(&self) -> Address {
        self.blockchain().get_owner_address()
    }

    fn try_convert_to_elrond_address(&self, address: &BoxedBytes) -> SCResult<Address> {
        require!(
            address.len() == Address::len_bytes(),
            "Wrong address format, it should be exactly 32 bytes"
        );

        Ok(Address::from_slice(address.as_slice()))
    }

    fn async_transfer_esdt(
        &self,
        to: Address,
        token_id: TokenIdentifier,
        amount: Self::BigUint,
    ) -> AsyncCall<Self::SendApi> {
        let contract_call_raw =
            ContractCall::<Self::SendApi, ()>::new(self.send(), to, BoxedBytes::empty())
                .with_token_transfer(token_id, amount);

        contract_call_raw.async_call()
    }

    fn try_mint(&self, token_id: &TokenIdentifier, amount: &Self::BigUint) -> SCResult<()> {
        self.require_local_mint_role_set(token_id)?;
        self.send().esdt_local_mint(token_id, 0, amount);

        Ok(())
    }

    fn try_burn(&self, token_id: &TokenIdentifier, amount: &Self::BigUint) -> SCResult<()> {
        self.require_local_burn_role_set(token_id)?;
        self.send().esdt_local_burn(token_id, 0, amount);

        Ok(())
    }

    fn require_local_mint_role_set(&self, token_id: &TokenIdentifier) -> SCResult<()> {
        let roles = self.blockchain().get_esdt_local_roles(token_id);

        require!(
            roles.contains(&EsdtLocalRole::Mint),
            "Local mint role not set"
        );

        Ok(())
    }

    fn require_local_burn_role_set(&self, token_id: &TokenIdentifier) -> SCResult<()> {
        let roles = self.blockchain().get_esdt_local_roles(token_id);

        require!(
            roles.contains(&EsdtLocalRole::Burn),
            "Local burn role not set"
        );

        Ok(())
    }

    // storage

    /// Elrond token ID to another chain's representation of that token (for example, an ERC20 contract address)
    #[storage_mapper("assetHashMap")]
    fn asset_hash_map(
        &self,
        token_id: &TokenIdentifier,
        to_chain_id: u64,
    ) -> SingleValueMapper<Self::Storage, BoxedBytes>;

    /// Chain ID to proxy contract, which is likely the CrossChainManagement SC on the other chain
    #[storage_mapper("proxyHashMap")]
    fn proxy_hash_map(&self, to_chain_id: u64) -> SingleValueMapper<Self::Storage, BoxedBytes>;

    // events

    #[event("lock_event")]
    fn lock_event(
        &self,
        #[indexed] from_asset: &TokenIdentifier,
        #[indexed] sender: &Address,
        #[indexed] to_chain_id: u64,
        #[indexed] to_asset_hash: &BoxedBytes,
        #[indexed] dest_address: &BoxedBytes,
        amount: &Self::BigUint,
    );

    #[event("unlock_event")]
    fn unlock_event(
        &self,
        #[indexed] to_asset: &TokenIdentifier,
        #[indexed] receiver: &Address,
        amount: &Self::BigUint,
    );

    // proxies

    #[proxy]
    fn cross_chain_management_proxy(
        &self,
        to: Address,
    ) -> cross_chain_management_proxy::Proxy<Self::SendApi>;
}
