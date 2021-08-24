#![no_std]

use transaction::TransactionArgs;

elrond_wasm::imports!();

#[elrond_wasm::contract]
pub trait TransactionRelayer {
    #[init]
    fn init(&self) {}

    // endpoints - owner-only

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

        let caller = self.blockchain().get_caller();
        let tx_args = TransactionArgs {
            dest_address,
            asset_hash: to_asset_hash,
            amount: payment_amount,
        };

        // TODO: Call CrossChainManagement SC to create the transaction
        let _ccm_address = self.get_cross_chain_management_sc_address();
        // require(eccm.crossChain(toChainId, toProxyHash, "unlock", txData), "EthCrossChainManager crossChain executed error!");

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
}
