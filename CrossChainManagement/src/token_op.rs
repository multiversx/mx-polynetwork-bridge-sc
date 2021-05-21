elrond_wasm::imports!();

#[elrond_wasm_derive::module]
pub trait TokenTransferModule {
    fn sc_async_transfer_esdt(
        &self,
        to: Address,
        token_id: TokenIdentifier,
        amount: Self::BigUint,
        func_name: BoxedBytes,
        args: &[BoxedBytes],
    ) -> AsyncCall<Self::SendApi> {
        let mut contract_call_raw =
            ContractCall::<Self::SendApi, ()>::new(self.send(), to, func_name)
                .with_token_transfer(token_id, amount);
        for arg in args {
            contract_call_raw.push_argument_raw_bytes(arg.as_slice());
        }

        contract_call_raw.async_call()
    }

    fn account_async_transfer_esdt(
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

    fn transfer_esdt(
        &self,
        dest: &Address,
        token_id: &TokenIdentifier,
        amount: &Self::BigUint,
        data: &[u8],
    ) {
        let _ = self.send().direct_esdt_via_transf_exec(
            dest,
            token_id.as_esdt_identifier(),
            amount,
            data,
        );
    }

    fn try_mint(&self, token_id: &TokenIdentifier, amount: &Self::BigUint) -> SCResult<()> {
        self.require_local_mint_role_set(&token_id)?;
        self.send().esdt_local_mint(
            self.blockchain().get_gas_left(),
            token_id.as_esdt_identifier(),
            amount,
        );

        Ok(())
    }

    fn try_burn(&self, token_id: &TokenIdentifier, amount: &Self::BigUint) -> SCResult<()> {
        self.require_local_burn_role_set(&token_id)?;
        self.send().esdt_local_burn(
            self.blockchain().get_gas_left(),
            token_id.as_esdt_identifier(),
            amount,
        );

        Ok(())
    }

    fn require_local_mint_role_set(&self, token_id: &TokenIdentifier) -> SCResult<()> {
        let roles = self
            .blockchain()
            .get_esdt_local_roles(token_id.as_esdt_identifier());

        require!(
            roles.contains(&EsdtLocalRole::Mint),
            "Local mint role not set"
        );

        Ok(())
    }

    fn require_local_burn_role_set(&self, token_id: &TokenIdentifier) -> SCResult<()> {
        let roles = self
            .blockchain()
            .get_esdt_local_roles(token_id.as_esdt_identifier());

        require!(
            roles.contains(&EsdtLocalRole::Burn),
            "Local burn role not set"
        );

        Ok(())
    }
}
